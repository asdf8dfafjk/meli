/*
 * meli - bin.rs
 *
 * Copyright 2017 Manos Pitsidianakis
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

pub mod ui;
use ui::*;

extern crate melib;
extern crate nom;
extern crate termion;
pub use melib::*;

use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;
use std::io::{stdout, stdin, };
use std::collections::VecDeque;
use std::time::{Duration, Instant};

fn main() {
    /* Lock all stdios */
    let _stdout = stdout();
    let mut _stdout = _stdout.lock();
    let stdin = stdin();
    let stdin = stdin;
    /*
       let _stderr = stderr();
       let mut _stderr = _stderr.lock();
       */



    let (sender, receiver): (SyncSender<ThreadEvent>, Receiver<ThreadEvent>) = sync_channel(::std::mem::size_of::<ThreadEvent>());
    {
        let mut cmd_queue = VecDeque::with_capacity(5);
        let sender = sender.clone();
        thread::Builder::new().name("input-thread".to_string()).spawn(move || {
            get_events(stdin, move | k| {
                //eprintln!("{:?}: queue is {:?}", Instant::now(), cmd_queue);
                let front: Option<(Instant, char)> = cmd_queue.front().map(|v: &(Instant, char)| { v.clone() });
                let back: Option<(Instant, char)> = cmd_queue.back().map(|v: &(Instant, char)| { v.clone() });
                let mut push: Option<(Instant, char)> = None;

                if let Key::Char(v) = k  {
                    if v == 'g' {
                        //eprintln!("{:?}: got 'g' in thread",Instant::now());
                        push = Some((Instant::now(), v));
                    } else if v > '/' && v < ':' {
                        //eprintln!("{:?}: got '{}' in thread", Instant::now(), v);
                        if let Some((_, 'g')) = front {
                            //eprintln!("{:?}: 'g' is front", Instant::now());
                            match back {
                                Some((i, cmd)) if cmd != 'g' => {
                                    let (i, cmd) = back.unwrap();
                                    let n = cmd as u8;
                                    //eprintln!("{:?}: check for num c={}, n={}", Instant::now(),cmd, n);
                                    if n > 0x2f && n < 0x3a {
                                        //eprintln!("{:?}: got a num {}", Instant::now(), cmd);
                                        let now = Instant::now();
                                        if now - i < Duration::from_millis(300) {
                                            push = Some((now,cmd));
                                            let ten_millis = Duration::from_millis(10);

                                            return;
                                        }
                                    }
                                },
                                Some((i, cmd)) => {
                                    let n = v as u8;
                                    //eprintln!("{:?}: check for num c={}, n={}", Instant::now(),v, n);
                                    if n > 0x2f && n < 0x3a {
                                        //eprintln!("{:?}: got a num {}", Instant::now(), v);
                                        let now = Instant::now();
                                        if now - i < Duration::from_millis(300) {
                                            push = Some((now,v));
                                        }
                                        cmd_queue.pop_front();
                                        let mut s = String::with_capacity(3);
                                        for (_, c) in cmd_queue.iter() {
                                            s.push(*c);
                                        }
                                        s.push(v);
                                        let times = s.parse::<usize>();
                                        //eprintln!("{:?}: parsed {:?}", Instant::now(), times);
                                        if let Ok(g) = times {
                                            sender.send(ThreadEvent::GoCmd(g)).unwrap();
                                            return;

                                        }
                                    }
                                },
                                None => {},
                            }


                        }
                    }
                    if let Some(v) = push {
                        cmd_queue.push_back(v);
                        return;


                    }
                }
                if push.is_none() {sender.send(ThreadEvent::Input(k)).unwrap();}
            })}).unwrap();
    }

    /*
    let folder_length = set.accounts["test_account"].folders.len();
    let mut account = Account::new("test_account".to_string(), set.accounts["test_account"].clone(), backends);
    
    {
        let sender = sender.clone();
        account.watch(RefreshEventConsumer::new(Box::new(move |r| {
            sender.send(ThreadEvent::from(r)).unwrap();
        })));
    }
    */
    let mut state = State::new(_stdout);

    let menu = Entity {component: Box::new(AccountMenu::new(&state.context.accounts)) };
    let listing = MailListing::new(Mailbox::new_dummy());
    let b = Entity { component: Box::new(listing) };
    let window  = Entity { component: Box::new(VSplit::new(menu,b,90)) };
    let status_bar = Entity { component: Box::new(StatusBar::new(window)) };
    state.register_entity(status_bar);

    let mut idxa = 0;
    let mut idxm = 0;
    let account_length = state.context.accounts.len();
    'main: loop {
        state.refresh_mailbox(idxa,idxm);
        let folder_length = state.context.accounts[idxa].len();
        state.render();

        'inner: loop {
            match receiver.recv().unwrap() {
                ThreadEvent::Input(k) => {
                    match k {
                        key @ Key::Char('j') | key @ Key::Char('k') => {
                            state.rcv_event(UIEvent { id: 0, event_type: UIEventType::Input(key)});
                            state.redraw();
                        },
                        key @ Key::Up | key @ Key::Down => {
                            state.rcv_event(UIEvent { id: 0, event_type: UIEventType::Input(key)});
                            state.redraw();
                        }
                        Key::Char('\n') => {
                            state.rcv_event(UIEvent { id: 0, event_type: UIEventType::Input(Key::Char('\n'))});
                            state.redraw();
                        }
                        Key::Char('i') | Key::Esc => {
                            state.rcv_event(UIEvent { id: 0, event_type: UIEventType::Input(Key::Esc)});
                            state.redraw();
                        }
                        Key::F(_) => {
                        },
                        Key::Char('q') | Key::Char('Q') => {
                            break 'main;
                        },
                        Key::Char('J') => if idxm + 1 < folder_length  {
                            idxm += 1;
                            break 'inner;
                        },
                        Key::Char('K') => if idxm > 0 {
                            idxm -= 1;
                            break 'inner;
                        },
                        Key::Char('l') => if idxa + 1 < account_length  {
                            idxa += 1;
                            idxm = 0;
                            break 'inner;
                        },
                        Key::Char('h') => if idxa > 0 {
                            idxa -= 1;
                            idxm = 0;
                            break 'inner;
                        },
                        Key::Char('r') => {
                            state.update_size();
                            state.render();
                        },
                        Key::Char(v) if v > '/' && v < ':' => {
                        },
                        _ => {}
                    }
                },
                ThreadEvent::RefreshMailbox { name : n } => {
                    eprintln!("Refresh mailbox {}", n);
                },
                ThreadEvent::UIEventType(e) => {
                    state.rcv_event(UIEvent { id: 0, event_type: e});
                    state.render();
                },
                ThreadEvent::GoCmd(v) => {
                    eprintln!("got go cmd with {:?}", v);
                },
            }
        }
    }
}
