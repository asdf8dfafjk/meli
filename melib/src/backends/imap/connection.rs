/*
 * meli - imap module.
 *
 * Copyright 2017 - 2019 Manos Pitsidianakis
 *
 * This file is part of meli.
 *
 * meli is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * meli is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with meli. If not, see <http://www.gnu.org/licenses/>.
 */

use super::protocol_parser::{ImapLineSplit, ImapResponse, RequiredResponses};
use crate::backends::MailboxHash;
use crate::connections::{lookup_ipv4, Connection};
use crate::email::parser::BytesExt;
use crate::error::*;
extern crate native_tls;
use futures::io::{AsyncReadExt, AsyncWriteExt};
use native_tls::TlsConnector;
pub use smol::Async as AsyncWrapper;
use std::collections::HashSet;
use std::future::Future;
use std::iter::FromIterator;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use super::protocol_parser;
use super::{Capabilities, ImapServerConf, UIDStore};

#[derive(Debug, Clone, Copy)]
pub enum ImapProtocol {
    IMAP { extension_use: ImapExtensionUse },
    ManageSieve,
}

#[derive(Debug, Clone, Copy)]
pub struct ImapExtensionUse {
    pub idle: bool,
    #[cfg(feature = "deflate_compression")]
    pub deflate: bool,
}

impl Default for ImapExtensionUse {
    fn default() -> Self {
        Self {
            idle: true,
            #[cfg(feature = "deflate_compression")]
            deflate: false,
        }
    }
}

#[derive(Debug)]
pub struct ImapStream {
    pub cmd_id: usize,
    pub stream: AsyncWrapper<Connection>,
    pub protocol: ImapProtocol,
    pub current_mailbox: MailboxSelection,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MailboxSelection {
    None,
    Select(MailboxHash),
    Examine(MailboxHash),
}

impl MailboxSelection {
    pub fn take(&mut self) -> Self {
        std::mem::replace(self, MailboxSelection::None)
    }
}

async fn try_await(cl: impl Future<Output = Result<()>> + Send) -> Result<()> {
    cl.await
}

#[derive(Debug)]
pub struct ImapConnection {
    pub stream: Result<ImapStream>,
    pub server_conf: ImapServerConf,
    pub uid_store: Arc<UIDStore>,
}

impl ImapStream {
    pub async fn new_connection(
        server_conf: &ImapServerConf,
    ) -> Result<(Capabilities, ImapStream)> {
        use std::net::TcpStream;
        let path = &server_conf.server_hostname;

        let cmd_id = 1;
        let stream = if server_conf.use_tls {
            let mut connector = TlsConnector::builder();
            if server_conf.danger_accept_invalid_certs {
                connector.danger_accept_invalid_certs(true);
            }
            let connector = connector
                .build()
                .chain_err_kind(crate::error::ErrorKind::Network)?;

            let addr = if let Ok(a) = lookup_ipv4(path, server_conf.server_port) {
                a
            } else {
                return Err(MeliError::new(format!(
                    "Could not lookup address {}",
                    &path
                )));
            };

            let mut socket = AsyncWrapper::new(Connection::Tcp(
                TcpStream::connect_timeout(&addr, std::time::Duration::new(4, 0))
                    .chain_err_kind(crate::error::ErrorKind::Network)?,
            ))
            .chain_err_kind(crate::error::ErrorKind::Network)?;
            if server_conf.use_starttls {
                let mut buf = vec![0; Connection::IO_BUF_SIZE];
                match server_conf.protocol {
                    ImapProtocol::IMAP { .. } => socket
                        .write_all(format!("M{} STARTTLS\r\n", cmd_id).as_bytes())
                        .await
                        .chain_err_kind(crate::error::ErrorKind::Network)?,
                    ImapProtocol::ManageSieve => {
                        socket
                            .read(&mut buf)
                            .await
                            .chain_err_kind(crate::error::ErrorKind::Network)?;
                        socket
                            .write_all(b"STARTTLS\r\n")
                            .await
                            .chain_err_kind(crate::error::ErrorKind::Network)?;
                    }
                }
                let mut response = String::with_capacity(1024);
                let mut broken = false;
                let now = std::time::Instant::now();

                while now.elapsed().as_secs() < 3 {
                    let len = socket
                        .read(&mut buf)
                        .await
                        .chain_err_kind(crate::error::ErrorKind::Network)?;
                    response.push_str(unsafe { std::str::from_utf8_unchecked(&buf[0..len]) });
                    match server_conf.protocol {
                        ImapProtocol::IMAP { .. } => {
                            if response.starts_with("* OK ") && response.find("\r\n").is_some() {
                                if let Some(pos) = response.as_bytes().find(b"\r\n") {
                                    response.drain(0..pos + 2);
                                }
                            }
                        }
                        ImapProtocol::ManageSieve => {
                            if response.starts_with("OK ") && response.find("\r\n").is_some() {
                                response.clear();
                                broken = true;
                                break;
                            }
                        }
                    }
                    if response.starts_with("M1 OK") {
                        broken = true;
                        break;
                    }
                }
                if !broken {
                    return Err(MeliError::new(format!(
                        "Could not initiate TLS negotiation to {}.",
                        path
                    )));
                }
            }

            {
                // FIXME: This is blocking
                let socket = socket
                    .into_inner()
                    .chain_err_kind(crate::error::ErrorKind::Network)?;
                let mut conn_result = connector.connect(path, socket);
                if let Err(native_tls::HandshakeError::WouldBlock(midhandshake_stream)) =
                    conn_result
                {
                    let mut midhandshake_stream = Some(midhandshake_stream);
                    loop {
                        match midhandshake_stream.take().unwrap().handshake() {
                            Ok(r) => {
                                conn_result = Ok(r);
                                break;
                            }
                            Err(native_tls::HandshakeError::WouldBlock(stream)) => {
                                midhandshake_stream = Some(stream);
                            }
                            p => {
                                p.chain_err_kind(crate::error::ErrorKind::Network)?;
                            }
                        }
                    }
                }
                AsyncWrapper::new(Connection::Tls(
                    conn_result.chain_err_kind(crate::error::ErrorKind::Network)?,
                ))
                .chain_err_kind(crate::error::ErrorKind::Network)?
            }
        } else {
            let addr = if let Ok(a) = lookup_ipv4(path, server_conf.server_port) {
                a
            } else {
                return Err(MeliError::new(format!(
                    "Could not lookup address {}",
                    &path
                )));
            };
            AsyncWrapper::new(Connection::Tcp(
                TcpStream::connect_timeout(&addr, std::time::Duration::new(4, 0))
                    .chain_err_kind(crate::error::ErrorKind::Network)?,
            ))
            .chain_err_kind(crate::error::ErrorKind::Network)?
        };
        let mut res = String::with_capacity(8 * 1024);
        let mut ret = ImapStream {
            cmd_id,
            stream,
            protocol: server_conf.protocol,
            current_mailbox: MailboxSelection::None,
        };
        if let ImapProtocol::ManageSieve = server_conf.protocol {
            use data_encoding::BASE64;
            ret.read_response(&mut res).await?;
            ret.send_command(
                format!(
                    "AUTHENTICATE \"PLAIN\" \"{}\"",
                    BASE64.encode(
                        format!(
                            "\0{}\0{}",
                            &server_conf.server_username, &server_conf.server_password
                        )
                        .as_bytes()
                    )
                )
                .as_bytes(),
            )
            .await?;
            ret.read_response(&mut res).await?;
            return Ok((Default::default(), ret));
        }

        ret.send_command(b"CAPABILITY").await?;
        ret.read_response(&mut res).await?;
        let capabilities: std::result::Result<Vec<&[u8]>, _> = res
            .split_rn()
            .find(|l| l.starts_with("* CAPABILITY"))
            .ok_or_else(|| MeliError::new(""))
            .and_then(|res| {
                protocol_parser::capabilities(res.as_bytes())
                    .map_err(|_| MeliError::new(""))
                    .map(|(_, v)| v)
            });

        if capabilities.is_err() {
            return Err(MeliError::new(format!(
                "Could not connect to {}: expected CAPABILITY response but got:{}",
                &server_conf.server_hostname, res
            )));
        }

        let capabilities = capabilities.unwrap();
        if !capabilities
            .iter()
            .any(|cap| cap.eq_ignore_ascii_case(b"IMAP4rev1"))
        {
            return Err(MeliError::new(format!(
                "Could not connect to {}: server is not IMAP4rev1 compliant",
                &server_conf.server_hostname
            )));
        } else if capabilities
            .iter()
            .any(|cap| cap.eq_ignore_ascii_case(b"LOGINDISABLED"))
        {
            return Err(MeliError::new(format!(
                "Could not connect to {}: server does not accept logins [LOGINDISABLED]",
                &server_conf.server_hostname
            ))
            .set_err_kind(crate::error::ErrorKind::Authentication));
        }

        let mut capabilities = None;
        ret.send_command(
            format!(
                "LOGIN \"{}\" \"{}\"",
                &server_conf.server_username, &server_conf.server_password
            )
            .as_bytes(),
        )
        .await?;
        let tag_start = format!("M{} ", (ret.cmd_id - 1));

        loop {
            ret.read_lines(&mut res, &String::new(), false).await?;
            let mut should_break = false;
            for l in res.split_rn() {
                if l.starts_with("* CAPABILITY") {
                    capabilities = protocol_parser::capabilities(l.as_bytes())
                        .map(|(_, capabilities)| {
                            HashSet::from_iter(capabilities.into_iter().map(|s: &[u8]| s.to_vec()))
                        })
                        .ok();
                }

                if l.starts_with(tag_start.as_str()) {
                    if !l[tag_start.len()..].trim().starts_with("OK ") {
                        return Err(MeliError::new(format!(
                            "Could not connect. Server replied with '{}'",
                            l[tag_start.len()..].trim()
                        ))
                        .set_err_kind(crate::error::ErrorKind::Authentication));
                    }
                    should_break = true;
                }
            }
            if should_break {
                break;
            }
        }

        if capabilities.is_none() {
            /* sending CAPABILITY after LOGIN automatically is an RFC recommendation, so check
             * for lazy servers */
            drop(capabilities);
            ret.send_command(b"CAPABILITY").await?;
            ret.read_response(&mut res).await.unwrap();
            let capabilities = protocol_parser::capabilities(res.as_bytes())?.1;
            let capabilities = HashSet::from_iter(capabilities.into_iter().map(|s| s.to_vec()));
            Ok((capabilities, ret))
        } else {
            let capabilities = capabilities.unwrap();
            Ok((capabilities, ret))
        }
    }

    pub async fn read_response(&mut self, ret: &mut String) -> Result<()> {
        let id = match self.protocol {
            ImapProtocol::IMAP { .. } => format!("M{} ", self.cmd_id - 1),
            ImapProtocol::ManageSieve => String::new(),
        };
        self.read_lines(ret, &id, true).await?;
        Ok(())
    }

    pub async fn read_lines(
        &mut self,
        ret: &mut String,
        termination_string: &str,
        keep_termination_string: bool,
    ) -> Result<()> {
        let mut buf: Vec<u8> = vec![0; Connection::IO_BUF_SIZE];
        ret.clear();
        let mut last_line_idx: usize = 0;
        loop {
            match self.stream.read(&mut buf).await {
                Ok(0) => break,
                Ok(b) => {
                    ret.push_str(unsafe { std::str::from_utf8_unchecked(&buf[0..b]) });
                    if let Some(mut pos) = ret[last_line_idx..].rfind("\r\n") {
                        if ret[last_line_idx..].starts_with("* BYE") {
                            return Err(MeliError::new("Disconnected"));
                        }
                        if let Some(prev_line) =
                            ret[last_line_idx..pos + last_line_idx].rfind("\r\n")
                        {
                            last_line_idx += prev_line + "\r\n".len();
                            pos -= prev_line + "\r\n".len();
                        }
                        if Some(pos + "\r\n".len()) == ret.get(last_line_idx..).map(|r| r.len()) {
                            if !termination_string.is_empty()
                                && ret[last_line_idx..].starts_with(termination_string)
                            {
                                debug!(&ret[last_line_idx..]);
                                if !keep_termination_string {
                                    ret.replace_range(last_line_idx.., "");
                                }
                                break;
                            } else if termination_string.is_empty() {
                                break;
                            }
                        }
                        last_line_idx += pos + "\r\n".len();
                    }
                }
                Err(e) => {
                    return Err(MeliError::from(e).set_err_kind(crate::error::ErrorKind::Network));
                }
            }
        }
        //debug!("returning IMAP response:\n{:?}", &ret);
        Ok(())
    }

    pub async fn wait_for_continuation_request(&mut self) -> Result<()> {
        let term = "+ ".to_string();
        let mut ret = String::new();
        self.read_lines(&mut ret, &term, false).await?;
        Ok(())
    }

    pub async fn send_command(&mut self, command: &[u8]) -> Result<()> {
        if let Err(err) = try_await(async move {
            let command = command.trim();
            match self.protocol {
                ImapProtocol::IMAP { .. } => {
                    self.stream.write_all(b"M").await?;
                    self.stream
                        .write_all(self.cmd_id.to_string().as_bytes())
                        .await?;
                    self.stream.write_all(b" ").await?;
                    self.cmd_id += 1;
                }
                ImapProtocol::ManageSieve => {}
            }

            self.stream.write_all(command).await?;
            self.stream.write_all(b"\r\n").await?;
            self.stream.flush().await?;
            match self.protocol {
                ImapProtocol::IMAP { .. } => {
                    debug!("sent: M{} {}", self.cmd_id - 1, unsafe {
                        std::str::from_utf8_unchecked(command)
                    });
                }
                ImapProtocol::ManageSieve => {}
            }
            Ok(())
        })
        .await
        {
            Err(err.set_err_kind(crate::error::ErrorKind::Network))
        } else {
            Ok(())
        }
    }

    pub async fn send_literal(&mut self, data: &[u8]) -> Result<()> {
        if let Err(err) = try_await(async move {
            self.stream.write_all(data).await?;
            self.stream.write_all(b"\r\n").await?;
            Ok(())
        })
        .await
        {
            Err(err.set_err_kind(crate::error::ErrorKind::Network))
        } else {
            Ok(())
        }
    }

    pub async fn send_raw(&mut self, raw: &[u8]) -> Result<()> {
        if let Err(err) = try_await(async move {
            self.stream.write_all(raw).await?;
            self.stream.write_all(b"\r\n").await?;
            Ok(())
        })
        .await
        {
            Err(err.set_err_kind(crate::error::ErrorKind::Network))
        } else {
            Ok(())
        }
    }
}

impl ImapConnection {
    pub fn new_connection(
        server_conf: &ImapServerConf,
        uid_store: Arc<UIDStore>,
    ) -> ImapConnection {
        ImapConnection {
            stream: Err(MeliError::new("Offline".to_string())),
            server_conf: server_conf.clone(),
            uid_store,
        }
    }

    pub fn connect<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            if let (instant, ref mut status @ Ok(())) = *self.uid_store.is_online.lock().unwrap() {
                if Instant::now().duration_since(instant) >= std::time::Duration::new(60 * 30, 0) {
                    *status = Err(MeliError::new("Connection timed out"));
                    self.stream = Err(MeliError::new("Connection timed out"));
                }
            }
            if self.stream.is_ok() {
                self.uid_store.is_online.lock().unwrap().0 = Instant::now();
                return Ok(());
            }
            let new_stream = debug!(ImapStream::new_connection(&self.server_conf).await);
            if let Err(err) = new_stream.as_ref() {
                *self.uid_store.is_online.lock().unwrap() = (Instant::now(), Err(err.clone()));
            } else {
                *self.uid_store.is_online.lock().unwrap() = (Instant::now(), Ok(()));
            }
            let (capabilities, stream) = new_stream?;
            self.stream = Ok(stream);
            match self.stream.as_ref()?.protocol {
                ImapProtocol::IMAP {
                    extension_use:
                        ImapExtensionUse {
                            #[cfg(feature = "deflate_compression")]
                            deflate,
                            idle: _idle,
                        },
                } =>
                {
                    #[cfg(feature = "deflate_compression")]
                    if capabilities.contains(&b"COMPRESS=DEFLATE"[..]) && deflate {
                        let mut ret = String::new();
                        self.send_command(b"COMPRESS DEFLATE").await?;
                        self.read_response(&mut ret, RequiredResponses::empty())
                            .await?;
                        match ImapResponse::from(&ret) {
                            ImapResponse::No(code)
                            | ImapResponse::Bad(code)
                            | ImapResponse::Preauth(code)
                            | ImapResponse::Bye(code) => {
                                crate::log(format!("Could not use COMPRESS=DEFLATE in account `{}`: server replied with `{}`", self.uid_store.account_name, code), crate::LoggingLevel::WARN);
                            }
                            ImapResponse::Ok(_) => {
                                let ImapStream {
                                    cmd_id,
                                    stream,
                                    protocol,
                                    current_mailbox,
                                } = std::mem::replace(&mut self.stream, Err(MeliError::new("")))?;
                                let stream = stream.into_inner()?;
                                self.stream = Ok(ImapStream {
                                    cmd_id,
                                    stream: AsyncWrapper::new(stream.deflate())?,
                                    protocol,
                                    current_mailbox,
                                });
                            }
                        }
                    }
                }
                ImapProtocol::ManageSieve => {}
            }
            *self.uid_store.capabilities.lock().unwrap() = capabilities;
            Ok(())
        })
    }

    pub fn read_response<'a>(
        &'a mut self,
        ret: &'a mut String,
        required_responses: RequiredResponses,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut response = String::new();
            ret.clear();
            self.stream.as_mut()?.read_response(&mut response).await?;

            match self.server_conf.protocol {
                ImapProtocol::IMAP { .. } => {
                    let r: ImapResponse = ImapResponse::from(&response);
                    match r {
                        ImapResponse::Bye(ref response_code) => {
                            self.stream = Err(MeliError::new(format!(
                                "Offline: received BYE: {:?}",
                                response_code
                            )));
                            ret.push_str(&response);
                        }
                        ImapResponse::No(ref response_code) => {
                            //FIXME return error
                            debug!("Received NO response: {:?} {:?}", response_code, response);
                            ret.push_str(&response);
                        }
                        ImapResponse::Bad(ref response_code) => {
                            //FIXME return error
                            debug!("Received BAD response: {:?} {:?}", response_code, response);
                            ret.push_str(&response);
                        }
                        _ => {
                            /*debug!(
                                "check every line for required_responses: {:#?}",
                                &required_responses
                            );*/
                            for l in response.split_rn() {
                                /*debug!("check line: {}", &l);*/
                                if required_responses.check(l) || !self.process_untagged(l).await? {
                                    ret.push_str(l);
                                }
                            }
                        }
                    }
                    r.into()
                }
                ImapProtocol::ManageSieve => {
                    ret.push_str(&response);
                    Ok(())
                }
            }
        })
    }

    pub async fn read_lines(&mut self, ret: &mut String, termination_string: String) -> Result<()> {
        self.stream
            .as_mut()?
            .read_lines(ret, &termination_string, false)
            .await?;
        Ok(())
    }

    pub async fn wait_for_continuation_request(&mut self) -> Result<()> {
        self.stream
            .as_mut()?
            .wait_for_continuation_request()
            .await?;
        Ok(())
    }

    pub async fn send_command(&mut self, command: &[u8]) -> Result<()> {
        if let Err(err) =
            try_await(async { self.stream.as_mut()?.send_command(command).await }).await
        {
            self.stream = Err(err.clone());
            if err.kind.is_network() {
                self.connect().await?;
            }
            Err(err)
        } else {
            Ok(())
        }
    }

    pub async fn send_literal(&mut self, data: &[u8]) -> Result<()> {
        if let Err(err) = try_await(async { self.stream.as_mut()?.send_literal(data).await }).await
        {
            self.stream = Err(err.clone());
            if err.kind.is_network() {
                self.connect().await?;
            }
            Err(err)
        } else {
            Ok(())
        }
    }

    pub async fn send_raw(&mut self, raw: &[u8]) -> Result<()> {
        if let Err(err) = try_await(async { self.stream.as_mut()?.send_raw(raw).await }).await {
            self.stream = Err(err.clone());
            if err.kind.is_network() {
                self.connect().await?;
            }
            Err(err)
        } else {
            Ok(())
        }
    }

    pub async fn select_mailbox(
        &mut self,
        mailbox_hash: MailboxHash,
        ret: &mut String,
        force: bool,
    ) -> Result<()> {
        if !force && self.stream.as_ref()?.current_mailbox == MailboxSelection::Select(mailbox_hash)
        {
            return Ok(());
        }
        self.send_command(
            format!(
                "SELECT \"{}\"",
                self.uid_store.mailboxes.lock().await[&mailbox_hash].imap_path()
            )
            .as_bytes(),
        )
        .await?;
        self.read_response(ret, RequiredResponses::SELECT_REQUIRED)
            .await?;
        debug!("select response {}", ret);
        self.stream.as_mut()?.current_mailbox = MailboxSelection::Select(mailbox_hash);
        Ok(())
    }

    pub async fn examine_mailbox(
        &mut self,
        mailbox_hash: MailboxHash,
        ret: &mut String,
        force: bool,
    ) -> Result<()> {
        if !force
            && self.stream.as_ref()?.current_mailbox == MailboxSelection::Examine(mailbox_hash)
        {
            return Ok(());
        }
        self.send_command(
            format!(
                "EXAMINE \"{}\"",
                self.uid_store.mailboxes.lock().await[&mailbox_hash].imap_path()
            )
            .as_bytes(),
        )
        .await?;
        self.read_response(ret, RequiredResponses::EXAMINE_REQUIRED)
            .await?;
        debug!("examine response {}", ret);
        self.stream.as_mut()?.current_mailbox = MailboxSelection::Examine(mailbox_hash);
        Ok(())
    }

    pub async fn unselect(&mut self) -> Result<()> {
        match self.stream.as_mut()?.current_mailbox.take() {
            MailboxSelection::Examine(mailbox_hash) |
            MailboxSelection::Select(mailbox_hash) =>{
            let mut response = String::with_capacity(8 * 1024);
            if self
                .uid_store
                .capabilities
                .lock()
                .unwrap()
                .iter()
                .any(|cap| cap.eq_ignore_ascii_case(b"UNSELECT"))
            {
                self.send_command(b"UNSELECT").await?;
                self.read_response(&mut response, RequiredResponses::empty())
                    .await?;
            } else {
                /* `RFC3691 - UNSELECT Command` states: "[..] IMAP4 provides this
                 * functionality (via a SELECT command with a nonexistent mailbox name or
                 * reselecting the same mailbox with EXAMINE command)[..]
                 */
                
                self.select_mailbox(mailbox_hash, &mut response, true).await?;
                self.examine_mailbox(mailbox_hash, &mut response, true).await?;
            }
        },
        MailboxSelection::None => {},
        }
        Ok(())
    }

    pub fn add_refresh_event(&mut self, ev: crate::backends::RefreshEvent) {
        if let Some(ref sender) = self.uid_store.sender.read().unwrap().as_ref() {
            sender.send(ev);
            for ev in self.uid_store.refresh_events.lock().unwrap().drain(..) {
                sender.send(ev);
            }
        } else {
            self.uid_store.refresh_events.lock().unwrap().push(ev);
        }
    }

    pub async fn create_uid_msn_cache(
        &mut self,
        mailbox_hash: MailboxHash,
        low: usize,
    ) -> Result<()> {
        debug_assert!(low > 0);
        let mut response = String::new();
        self.examine_mailbox(mailbox_hash, &mut response, false)
            .await?;
        self.send_command(format!("UID SEARCH {}:*", low).as_bytes())
            .await?;
        self.read_response(&mut response, RequiredResponses::SEARCH)
            .await?;
        debug!("uid search response {:?}", &response);
        let mut msn_index_lck = self.uid_store.msn_index.lock().unwrap();
        let msn_index = msn_index_lck.entry(mailbox_hash).or_default();
        let _ = msn_index.drain(low - 1..);
        msn_index.extend(
            debug!(protocol_parser::search_results(response.as_bytes()))?
                .1
                .into_iter(),
        );
        Ok(())
    }
}

pub struct ImapBlockingConnection {
    buf: Vec<u8>,
    result: Vec<u8>,
    prev_res_length: usize,
    pub conn: ImapConnection,
    err: Option<String>,
}

impl From<ImapConnection> for ImapBlockingConnection {
    fn from(conn: ImapConnection) -> Self {
        ImapBlockingConnection {
            buf: vec![0; Connection::IO_BUF_SIZE],
            conn,
            prev_res_length: 0,
            result: Vec::with_capacity(8 * 1024),
            err: None,
        }
    }
}

impl ImapBlockingConnection {
    pub fn into_conn(self) -> ImapConnection {
        self.conn
    }

    pub fn err(&self) -> Option<&str> {
        self.err.as_ref().map(String::as_str)
    }

    pub fn as_stream<'a>(&'a mut self) -> impl Future<Output = Option<Vec<u8>>> + 'a {
        self.result.drain(0..self.prev_res_length);
        self.prev_res_length = 0;
        let mut break_flag = false;
        let mut prev_failure = None;
        async move {
            if self.conn.stream.is_err() {
                debug!(&self.conn.stream);
                return None;
            }
            loop {
                if let Some(y) = read(self, &mut break_flag, &mut prev_failure).await {
                    return Some(y);
                }
                if break_flag {
                    return None;
                }
            }
        }
    }
}

async fn read(
    conn: &mut ImapBlockingConnection,
    break_flag: &mut bool,
    prev_failure: &mut Option<std::time::Instant>,
) -> Option<Vec<u8>> {
    let ImapBlockingConnection {
        ref mut prev_res_length,
        ref mut result,
        ref mut conn,
        ref mut buf,
        ref mut err,
    } = conn;

    match conn.stream.as_mut().unwrap().stream.read(buf).await {
        Ok(0) => {
            *break_flag = true;
        }
        Ok(b) => {
            result.extend_from_slice(&buf[0..b]);
            debug!(unsafe { std::str::from_utf8_unchecked(result) });
            if let Some(pos) = result.find(b"\r\n") {
                *prev_res_length = pos + b"\r\n".len();
                return Some(result[0..*prev_res_length].to_vec());
            }
            *prev_failure = None;
        }
        Err(e) => {
            debug!(&conn.stream);
            debug!(&e);
            *err = Some(e.to_string());
            *break_flag = true;
            *prev_failure = Some(Instant::now());
        }
    }
    None
}
