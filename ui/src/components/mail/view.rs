use super::*;
use linkify::{LinkFinder, Link};
use std::process::{Command, Stdio};

use mime_apps::query_default_app;


#[derive(PartialEq, Debug)]
enum ViewMode {
    Normal,
    Url,
   // Attachment,
   // Raw,
}

/// Contains an Envelope view, with sticky headers, a pager for the body, and subviews for more
/// menus
pub struct MailView {
    coordinates: (usize, usize, usize),
    pager: Option<Pager>,
    subview: Option<Box<MailView>>,
    dirty: bool,
    mode: ViewMode,

    cmd_buf: String,
}

impl MailView {
    pub fn new(coordinates: (usize, usize, usize), pager: Option<Pager>, subview: Option<Box<MailView>>) -> Self {

        MailView {
            coordinates: coordinates,
            pager: pager,
            subview: subview,
            dirty: true,
            mode: ViewMode::Normal,

            cmd_buf: String::with_capacity(4),
        }
    }
}


impl Component for MailView {
    fn draw(&mut self, grid: &mut CellBuffer, area: Area, context: &mut Context) {
        let upper_left = upper_left!(area);
        let bottom_right = bottom_right!(area);

        let (envelope_idx, y): (usize, usize) = {
            let threaded = context.accounts[self.coordinates.0].runtime_settings.threaded;
            let mailbox = &mut context.accounts[self.coordinates.0][self.coordinates.1].as_ref().unwrap().as_ref().unwrap();
            let envelope_idx: usize = if threaded {
                mailbox.threaded_mail(self.coordinates.2)
            } else {
                self.coordinates.2
            };

            let envelope: &Envelope = &mailbox.collection[envelope_idx];

            let (x,y) = write_string_to_grid(&format!("Date: {}", envelope.date_as_str()),
            grid,
            Color::Byte(33),
            Color::Default,
            area,
            true);
            for x in x..=get_x(bottom_right) {
                grid[(x, y)].set_ch(' ');
                grid[(x, y)].set_bg(Color::Default);
                grid[(x, y)].set_fg(Color::Default);
            }
            let (x,y) = write_string_to_grid(&format!("From: {}", envelope.from()),
            grid,
            Color::Byte(33),
            Color::Default,
            (set_y(upper_left, y+1), bottom_right),
            true);
            for x in x..=get_x(bottom_right) {
                grid[(x, y)].set_ch(' ');
                grid[(x, y)].set_bg(Color::Default);
                grid[(x, y)].set_fg(Color::Default);
            }
            let (x,y) = write_string_to_grid(&format!("To: {}", envelope.to()),
            grid,
            Color::Byte(33),
            Color::Default,
            (set_y(upper_left, y+1), bottom_right),
            true);
            for x in x..=get_x(bottom_right) {
                grid[(x, y)].set_ch(' ');
                grid[(x, y)].set_bg(Color::Default);
                grid[(x, y)].set_fg(Color::Default);
            }
            let (x,y) = write_string_to_grid(&format!("Subject: {}", envelope.subject()),
            grid,
            Color::Byte(33),
            Color::Default,
            (set_y(upper_left, y+1), bottom_right),
            true);
            for x in x..=get_x(bottom_right) {
                grid[(x, y)].set_ch(' ');
                grid[(x, y)].set_bg(Color::Default);
                grid[(x, y)].set_fg(Color::Default);
            }
            let (x, y) = write_string_to_grid(&format!("Message-ID: {}", envelope.message_id_raw()),
            grid,
            Color::Byte(33),
            Color::Default,
            (set_y(upper_left, y+1), bottom_right),
            true);
            for x in x..=get_x(bottom_right) {
                grid[(x, y)].set_ch(' ');
                grid[(x, y)].set_bg(Color::Default);
                grid[(x, y)].set_fg(Color::Default);
            }
            clear_area(grid,
                       (set_y(upper_left, y+1), set_y(bottom_right, y+2)));
            context.dirty_areas.push_back((upper_left, set_y(bottom_right, y+1)));
            (envelope_idx, y + 1)
        };

        if self.dirty {
            let buf = {
                let mailbox_idx = self.coordinates;  // coordinates are mailbox idxs
                let mailbox = &mut context.accounts[mailbox_idx.0][mailbox_idx.1].as_ref().unwrap().as_ref().unwrap();
                let envelope : &Envelope = &mailbox.collection[envelope_idx];

                let finder = LinkFinder::new();
                let mut t = envelope.body().text().to_string();
                let mut text = match self.mode {
                    ViewMode::Url => {
                        for (lidx, l) in finder.links(&envelope.body().text()).enumerate() {
                            t.insert_str(l.start()+(lidx*3), &format!("[{}]", lidx));
                        }
                        t
                    },
                    _ => envelope.body().text().to_string()
                };
                if envelope.body().count_attachments() > 1 {
                    text = envelope.body().attachments().iter().enumerate().fold(text, |mut s, (idx, a)| { s.push_str(&format!("[{}] {}\n\n", idx, a)); s });
                }
                let mut buf = CellBuffer::from(&text);
                match self.mode {
                    ViewMode::Url => {
                        // URL indexes must be colored (ugh..)
                        let lines: Vec<&str> = text.split('\n').collect();
                        let mut shift = 0;
                        for r in lines.iter() {
                            for l in finder.links(&r) {
                                buf[(l.start() + shift - 1, 0)].set_fg(Color::Byte(226));
                                buf[(l.start() + shift - 2, 0)].set_fg(Color::Byte(226));
                                buf[(l.start() + shift - 3, 0)].set_fg(Color::Byte(226));
                            }
                            // Each Cell represents one char so next line will be:
                            shift += r.chars().count()+1;
                        }
                    },
                    _ => {},
                }
               buf
            };
            let cursor_pos = self.pager.as_mut().map(|p| p.cursor_pos());
            // TODO: pass string instead of envelope
            self.pager = Some(Pager::from_buf(buf, cursor_pos));
            self.dirty = false;
        }
        self.pager.as_mut().map(|p| p.draw(grid, (set_y(upper_left, y + 1),bottom_right), context));
    }

    fn process_event(&mut self, event: &UIEvent, context: &mut Context) {
        match event.event_type {
            UIEventType::Input(Key::Esc) => {
                match self.mode {
                    ViewMode::Url => {
                        self.cmd_buf.clear();
                        return;
                    },
                    _ => {},
                }
            },
            UIEventType::Input(Key::Char(c)) if c >= '0' && c <= '9' => { //TODO:this should be an Action
                self.cmd_buf.push(c);
            },
            UIEventType::Input(Key::Char('a')) if self.cmd_buf.len() > 0 && self.mode == ViewMode::Normal => { //TODO:this should be an Action
                let lidx = self.cmd_buf.parse::<usize>().unwrap();
                self.cmd_buf.clear();


                {
                    let threaded = context.accounts[self.coordinates.0].runtime_settings.threaded;
                    let mailbox = &mut context.accounts[self.coordinates.0][self.coordinates.1].as_ref().unwrap().as_ref().unwrap();
                    let envelope_idx: usize = if threaded {
                        mailbox.threaded_mail(self.coordinates.2)
                    } else {
                        self.coordinates.2
                    };

                    let envelope: &Envelope = &mailbox.collection[envelope_idx];
                    if let Some(u) = envelope.body().attachments().get(lidx) {
                        let mut p = create_temp_file(&decode(u), None);
                        let attachment_type = u.mime_type();
                        eprintln!("attachment type {}", attachment_type);
                        let binary = query_default_app(attachment_type);
                        eprintln!("{:?}, binary = {:?}", p, binary);
                Command::new(binary.unwrap())
                    .arg(p.path())
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .expect("Failed to start xdg_open");

                    } else {
                        context.replies.push_back(UIEvent { id: 0, event_type: UIEventType::StatusNotification(format!("Attachment `{}` not found.", lidx)) });
                        return;
                    }
                };


            },
            UIEventType::Input(Key::Char('g')) if self.cmd_buf.len() > 0 && self.mode == ViewMode::Url => { //TODO:this should be an Action

                let lidx = self.cmd_buf.parse::<usize>().unwrap();
                self.cmd_buf.clear();
                let url = {
                    let threaded = context.accounts[self.coordinates.0].runtime_settings.threaded;
                    let mailbox = &mut context.accounts[self.coordinates.0][self.coordinates.1].as_ref().unwrap().as_ref().unwrap();
                    let envelope_idx: usize = if threaded {
                        mailbox.threaded_mail(self.coordinates.2)
                    } else {
                        self.coordinates.2
                    };

                    let envelope: &Envelope = &mailbox.collection[envelope_idx];
                    let finder = LinkFinder::new();
                    let mut t = envelope.body().text().to_string();
                    let links: Vec<Link> = finder.links(&t).collect();
                    if let Some(u) = links.get(lidx) {
                        u.as_str().to_string()
                    } else {
                        context.replies.push_back(UIEvent { id: 0, event_type: UIEventType::StatusNotification(format!("Link `{}` not found.", lidx)) });
                        return;
                    }
                };


                Command::new("xdg-open")
                    .arg(url)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .expect("Failed to start xdg_open");

            },
            UIEventType::Input(Key::Char('u')) => { //TODO:this should be an Action
                match self.mode {
                    ViewMode::Normal => { self.mode = ViewMode::Url },
                    ViewMode::Url => { self.mode = ViewMode::Normal },
                    //_ => {},
                }
                self.dirty = true;
            },
            _ => {},
        }
        if let Some(ref mut sub) = self.subview {
            sub.process_event(event, context);

        } else {
            if let Some(ref mut p) = self.pager {
                p.process_event(event, context);
            }
        }
    }
    fn is_dirty(&self) -> bool {
        self.dirty || self.pager.as_ref().map(|p| p.is_dirty()).unwrap_or(false) ||
            self.subview.as_ref().map(|p| p.is_dirty()).unwrap_or(false)
    }
}
