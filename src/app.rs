use iced::widget::{button, column, container, row, scrollable, text, text_editor, text_input};
use iced::alignment::Vertical;
use iced::{Element, Length, Task};

use crate::asm::{assemble, Assembly, AsmError};
use crate::config::Config;
use crate::net::{NetError, UltimateClient, VerifyReport};
use crate::ui::hex::{format_bytes, parse_addr};
use crate::ui::settings::ConnectionStatus;

pub struct State {
    pub source: text_editor::Content,
    pub target_text: String,
    pub assembly: Option<Assembly>,
    pub errors: Vec<AsmError>,
    pub status: Status,
    pub config: Config,
    /// Session-only device host. Never persisted to `Config` or disk.
    pub host: String,
    /// Session-only network password. Never persisted to `Config` or disk.
    pub password: String,
    pub sending: bool,
    pub show_settings: bool,
    pub connection: Option<crate::ui::settings::ConnectionStatus>,
    pub mem_addr_text: String,
    pub mem_len_text: String,
    pub mem_rows: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SourceEdited(text_editor::Action),
    TargetChanged(String),
    Assemble,
    Send,
    SendFinished(Result<VerifyReport, NetError>),
    ToggleSettings,
    HostChanged(String),
    PasswordChanged(String),
    TestConnection,
    ConnectionTested(Result<(String, crate::net::DeviceInfo), NetError>),
    MemAddrChanged(String),
    MemLenChanged(String),
    MemRead,
    MemLoaded(Result<(u16, Vec<u8>), NetError>),
    OpenFile,
    FileOpened(Result<(std::path::PathBuf, String), String>),
    SaveFile,
    FileSaved(Result<std::path::PathBuf, String>),
    Clear,
    Exit,
}

impl Default for State {
    fn default() -> Self {
        Self {
            // Start empty. A canned program looked like the user's own work
            // and had to be cleared before every real session.
            source: text_editor::Content::new(),
            target_text: "C000".to_string(),
            assembly: None,
            errors: Vec::new(),
            status: "Ready".into(),
            config: Config::load(),
            host: String::new(),
            password: String::new(),
            sending: false,
            show_settings: true,
            connection: None,
            mem_addr_text: "0400".into(),
            mem_len_text: "256".into(),
            mem_rows: Vec::new(),
        }
    }
}

/// A status-bar message together with how it should be rendered.
///
/// Text and emphasis travel as one value on purpose. The obvious alternative —
/// a `String` plus a separate "is this good news" flag — means every one of the
/// seventeen places that set a status has to remember to set the flag too, and
/// the one that forgets leaves the previous message's colour behind. Making the
/// kind part of the value means it cannot be forgotten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub text: String,
    pub kind: StatusKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    /// Ordinary progress and results.
    Plain,
    /// Something worth noticing succeeded. Rendered bold green.
    Good,
}

impl Status {
    pub fn good(text: impl Into<String>) -> Self {
        Self { text: text.into(), kind: StatusKind::Good }
    }
}

impl<T: Into<String>> From<T> for Status {
    fn from(text: T) -> Self {
        Self { text: text.into(), kind: StatusKind::Plain }
    }
}

/// Shared width for the settings form's labels, so "Host:" and "Password:"
/// occupy the same column and their inputs share a left edge. Sized to fit the
/// longer of the two.
const LABEL_COLUMN: Length = Length::Fixed(90.0);

/// Shared width for the settings form's inputs. Sized for a full IPv4 address
/// ("255.255.255.255"); the password field matches it for consistency. Longer
/// entries still work — the field scrolls.
const FIELD_WIDTH: Length = Length::Fixed(170.0);

/// Bold weight for the connection result, which needs to stand out.
const BOLD: iced::Font = iced::Font {
    weight: iced::font::Weight::Bold,
    ..iced::Font::DEFAULT
};
/// Bright green, legible on the dark theme.
const OK_GREEN: iced::Color = iced::Color::from_rgb(0.25, 0.85, 0.35);
/// Red, tuned for the dark theme — pure red reads muddy against it.
const FAIL_RED: iced::Color = iced::Color::from_rgb(1.0, 0.35, 0.35);

/// Widget id for the host field, so the boot task can focus it.
fn host_input_id() -> iced::widget::Id {
    iced::widget::Id::new("host-input")
}

/// Initial state plus the startup task.
///
/// The settings panel opens on launch with the host field focused. The host is
/// deliberately never persisted, so every session begins by typing one, and
/// there is nothing useful to do in the app until it is entered — opening
/// buried behind a gear icon made the first action of every session a hunt.
pub fn boot() -> (State, Task<Message>) {
    (
        State::default(),
        iced::widget::operation::focus(host_input_id()),
    )
}

impl State {
    /// True when there is an assembled program with at least one byte in it.
    ///
    /// An empty buffer, a comment-only buffer, and a buffer of blank lines all
    /// assemble successfully to zero bytes — the assembler is right to accept
    /// them — so `assembly.is_some()` is not enough to mean "there is something
    /// to send". Both the Send button and the `Message::Send` guard read this,
    /// rather than each spelling out the condition, so they cannot drift apart.
    fn has_bytes_to_send(&self) -> bool {
        self.assembly.as_ref().is_some_and(|a| !a.bytes.is_empty())
    }

    /// True when there is anything for Clear to act on: text in the editor, a
    /// previous assembly, or an error list. Keeps the button dark when pressing
    /// it would be a no-op, matching Send and Connect.
    fn has_anything_to_clear(&self) -> bool {
        !self.source.text().trim().is_empty()
            || self.assembly.is_some()
            || !self.errors.is_empty()
    }

    /// Validates the host and builds an `UltimateClient` for it. All three
    /// device-facing message arms (`Send`, `TestConnection`, `MemRead`) go
    /// through this, so a blank host produces one clear message instead of
    /// each arm independently building `http://` + "" and surfacing a
    /// confusing `NetError::Transport` URL-parse error from reqwest.
    fn client(&self) -> Result<UltimateClient, String> {
        if self.host.trim().is_empty() {
            return Err("No host configured — open settings".into());
        }
        Ok(UltimateClient::new(
            &self.host,
            (!self.password.is_empty()).then(|| self.password.clone()),
        ))
    }
}

/// Formats the outcome of a `write_and_verify` call for the status bar.
pub fn send_summary(result: &Result<VerifyReport, NetError>) -> String {
    match result {
        Ok(r) => match &r.mismatch {
            None => format!(
                "{} bytes written to ${:04X} ({}) — verified",
                r.written, r.address, r.address
            ),
            Some(m) => format!(
                "Verify FAILED at ${:04X}: sent {:02X}, read back {:02X}",
                r.address as usize + m.offset,
                m.expected,
                m.actual
            ),
        },
        Err(e) => e.to_string(),
    }
}

/// Renders an assembly as `ADDR  BYTES  SOURCE`, one row per emitted line.
pub fn listing_text(assembly: &Assembly, source: &str) -> String {
    let src_lines: Vec<&str> = source.lines().collect();
    assembly
        .lines
        .iter()
        .map(|l| {
            let bytes = format_bytes(&assembly.bytes[l.start..l.start + l.len]);
            let text = src_lines
                .get(l.source_line - 1)
                .map(|s| s.split(';').next().unwrap_or("").trim())
                .unwrap_or("");
            format!("{:04X}  {:<8}  {}", l.address, bytes, text)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Invalidates any previously computed `Assembly` and its error list. Must be
/// called from every place that changes the source buffer or the target
/// address, so `state.assembly` never outlives the text/address it was built
/// from. `Message::Send` reads bytes and origin straight off `state.assembly`
/// without re-checking `state.target_text`, and `view()` pairs
/// `state.assembly`'s byte spans against the *current* editor buffer by line
/// index — either one is silently wrong if `state.assembly` is stale.
fn clear_stale_assembly(state: &mut State) {
    state.assembly = None;
    state.errors.clear();
}

pub fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::SourceEdited(action) => {
            // `text_editor::Action` covers nine variants — cursor motion,
            // selection, click, drag, scroll — and only `Edit(_)` actually
            // changes the buffer text. `on_action` forwards all nine (that is
            // how the cursor moves at all), so this message fires on every
            // arrow key and mouse click over the editor, not just typing.
            // Capture whether this one is a real edit *before* `perform`
            // consumes it, and only then invalidate the previous `Assembly`
            // — otherwise clicking in the editor after a successful Assemble
            // would blank the listing pane, and moving the cursor toward a
            // reported error line would blank the error list before the user
            // can act on it. See `clear_stale_assembly`.
            let edits = action.is_edit();
            state.source.perform(action);
            if edits {
                clear_stale_assembly(state);
            }
            Task::none()
        }
        Message::TargetChanged(t) => {
            state.target_text = t;
            // A new target address invalidates any previous `Assembly`, which
            // was built against the old origin. See `clear_stale_assembly`.
            clear_stale_assembly(state);
            Task::none()
        }
        Message::Assemble => {
            let Some(org) = parse_addr(&state.target_text) else {
                state.status = format!("'{}' is not a valid address", state.target_text).into();
                return Task::none();
            };
            // Tidy first, then assemble what the user will actually see, so
            // the listing's source column matches the editor. Only rewrite the
            // buffer when something changed — replacing the Content resets the
            // cursor, and doing that on every Assemble would be its own
            // annoyance.
            let typed = state.source.text();
            let src = crate::asm::format::uppercase_mnemonics(&typed);
            if src != typed {
                state.source = text_editor::Content::with_text(&src);
            }
            match assemble(&src, org) {
                Ok(a) => {
                    // Decimal as well as hex: the address is what gets typed
                    // into `SYS <n>` on the C64, and converting by hand every
                    // time is exactly the sort of chore a tool should absorb.
                    state.status = format!(
                        "{} bytes assembled at ${org:04X} ({org})",
                        a.bytes.len()
                    )
                    .into();
                    state.errors.clear();
                    state.assembly = Some(a);
                }
                Err(errs) => {
                    state.status = format!("{} error(s)", errs.len()).into();
                    state.errors = errs;
                    state.assembly = None;
                }
            }
            Task::none()
        }
        Message::Send => {
            let Some(a) = state.assembly.as_ref() else {
                state.status = "Nothing assembled — press Assemble first".into();
                return Task::none();
            };
            // The button is disabled in this case, but the guard is here too:
            // a UI gate is a convenience, not an invariant, and a zero-byte
            // POST would be reported back as "0 bytes written — verified",
            // which is true and completely useless.
            if a.bytes.is_empty() {
                state.status = "Nothing to send — the program is empty".into();
                return Task::none();
            }
            let client = match state.client() {
                Ok(c) => c,
                Err(e) => {
                    state.status = e.into();
                    return Task::none();
                }
            };
            // Only the *start* of the range is checked here — sound because
            // `assemble()` (src/asm/encode.rs) already rejects any program
            // where `org + len > 0x10000`, so a program that starts at or
            // above $0002 is guaranteed not to wrap back around and touch
            // $0000/$0001. See `send_guard_rejects_writes_that_touch_the_6510_io_port`.
            if (a.org as usize) < 2 {
                state.status =
                    "$0000/$0001 are the 6510's on-chip port and cannot be written by DMA".into();
                return Task::none();
            }

            let org = a.org;
            let bytes = a.bytes.clone();
            state.sending = true;
            state.status = format!("Sending {} bytes to ${org:04X}…", bytes.len()).into();

            Task::perform(
                async move { client.write_and_verify(org, &bytes).await },
                Message::SendFinished,
            )
        }
        Message::SendFinished(result) => {
            state.sending = false;
            state.status = send_summary(&result).into();
            Task::none()
        }
        Message::ToggleSettings => {
            state.show_settings = !state.show_settings;
            Task::none()
        }
        Message::HostChanged(h) => {
            // Held in memory only — never written to Config.
            state.host = h;
            Task::none()
        }
        Message::PasswordChanged(p) => {
            // Held in memory only — never written to Config.
            state.password = p;
            Task::none()
        }
        Message::TestConnection => {
            let client = match state.client() {
                Ok(c) => c,
                Err(e) => {
                    state.connection = Some(ConnectionStatus::Failed(e));
                    return Task::none();
                }
            };
            state.connection = Some(crate::ui::settings::ConnectionStatus::Testing);
            Task::perform(
                async move {
                    let v = client.version().await?;
                    let i = client.info().await?;
                    Ok((v, i))
                },
                Message::ConnectionTested,
            )
        }
        Message::ConnectionTested(result) => {
            match crate::ui::settings::connection_summary(&result) {
                // Success: the panel has done its job, so get it out of the
                // way and carry the good news down to the status bar. Leaving
                // it open would keep the editor pushed down for a result the
                // user has already read.
                ConnectionStatus::Connected(msg) => {
                    state.status = Status::good(msg);
                    state.connection = None;
                    state.show_settings = false;
                }
                // Failure: stay put. The fix is in this panel — usually the
                // host field, sometimes the password — so closing it would
                // hide the very controls the message is asking the user to
                // correct.
                other => state.connection = Some(other),
            }
            Task::none()
        }
        Message::MemAddrChanged(t) => {
            state.mem_addr_text = t;
            Task::none()
        }
        Message::MemLenChanged(t) => {
            state.mem_len_text = t;
            Task::none()
        }
        Message::MemRead => {
            let Some(addr) = parse_addr(&state.mem_addr_text) else {
                state.status = format!("'{}' is not a valid address", state.mem_addr_text).into();
                return Task::none();
            };
            let Some(len) = crate::ui::memview::parse_length(&state.mem_len_text) else {
                state.status = "Length must be 1 to 65536".into();
                return Task::none();
            };
            let client = match state.client() {
                Ok(c) => c,
                Err(e) => {
                    state.status = e.into();
                    return Task::none();
                }
            };
            Task::perform(
                async move { client.read_mem(addr, len).await.map(|b| (addr, b)) },
                Message::MemLoaded,
            )
        }
        Message::MemLoaded(Ok((addr, bytes))) => {
            let len = bytes.len();
            state.mem_rows = crate::ui::hex::hex_dump(addr, &bytes);
            state.status = format!("Read {len} bytes from ${addr:04X}").into();
            Task::none()
        }
        Message::MemLoaded(Err(e)) => {
            state.mem_rows.clear();
            state.status = e.to_string().into();
            Task::none()
        }
        Message::OpenFile => {
            let start = state.config.last_dir.clone();
            Task::perform(
                async move {
                    let mut dialog =
                        rfd::AsyncFileDialog::new().add_filter("assembly", &["asm", "s", "a65"]);
                    if let Some(dir) = start {
                        dialog = dialog.set_directory(dir);
                    }
                    let handle = dialog.pick_file().await.ok_or_else(|| "cancelled".to_string())?;
                    let path = handle.path().to_path_buf();
                    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                    Ok((path, text))
                },
                Message::FileOpened,
            )
        }
        Message::FileOpened(Ok((path, text))) => {
            state.source = text_editor::Content::with_text(&text);
            clear_stale_assembly(state);
            state.status = format!("Opened {}", path.display()).into();
            // Update in-memory state synchronously (this is what the tests
            // exercise); persist `last_dir` to disk via a separate `Task` so
            // `update` itself stays pure and `cargo test` never touches the
            // real config directory — tests call `update` directly and drop
            // the returned `Task` without polling it.
            if let Some(dir) = path.parent() {
                state.config.last_dir = Some(dir.to_path_buf());
                let config = state.config.clone();
                return Task::future(async move {
                    let _ = config.save();
                })
                .discard();
            }
            Task::none()
        }
        Message::FileOpened(Err(e)) => {
            if e != "cancelled" {
                state.status = e.into();
            }
            Task::none()
        }
        Message::SaveFile => {
            let start = state.config.last_dir.clone();
            let text = state.source.text();
            Task::perform(
                async move {
                    let mut dialog = rfd::AsyncFileDialog::new().set_file_name("program.asm");
                    if let Some(dir) = start {
                        dialog = dialog.set_directory(dir);
                    }
                    let handle = dialog.save_file().await.ok_or_else(|| "cancelled".to_string())?;
                    let path = handle.path().to_path_buf();
                    std::fs::write(&path, text).map_err(|e| e.to_string())?;
                    Ok(path)
                },
                Message::FileSaved,
            )
        }
        Message::FileSaved(Ok(path)) => {
            state.status = format!("Saved {}", path.display()).into();
            // See the comment in `FileOpened(Ok(..))`: state mutation is
            // synchronous and testable, the disk write is a separate `Task`.
            if let Some(dir) = path.parent() {
                state.config.last_dir = Some(dir.to_path_buf());
                let config = state.config.clone();
                return Task::future(async move {
                    let _ = config.save();
                })
                .discard();
            }
            Task::none()
        }
        Message::Clear => {
            // Editor and both output panes. The memory viewer is left alone —
            // it shows what was read off the device, not anything produced
            // from this buffer.
            state.source = text_editor::Content::new();
            state.assembly = None;
            state.errors.clear();
            state.status = "Cleared".into();
            Task::none()
        }
        Message::Exit => iced::exit(),
        Message::FileSaved(Err(e)) => {
            if e != "cancelled" {
                state.status = e.into();
            }
            Task::none()
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let toolbar = row![
        button("Open").on_press(Message::OpenFile),
        button("Save").on_press(Message::SaveFile),
        button("Clear")
            .on_press_maybe(state.has_anything_to_clear().then_some(Message::Clear)),
        text("Target: $"),
        text_input("C000", &state.target_text)
            .on_input(Message::TargetChanged)
            .width(Length::Fixed(90.0)),
        button("Assemble").on_press(Message::Assemble),
        button("Send").on_press_maybe(
            (!state.sending && state.has_bytes_to_send()).then_some(Message::Send),
        ),
        button("⚙").on_press(Message::ToggleSettings),
        // Last in the row: it is the one action that ends the session.
        button("Exit").on_press(Message::Exit),
    ]
    .spacing(8)
    .align_y(Vertical::Center);

    let settings: Element<Message> = if state.show_settings {
        column![
            row![
                // "Host:" and "Password:" are different lengths, so without a
                // fixed label column each field starts wherever its own label
                // happens to end. One width for both puts the inputs on a
                // common left edge.
                text("Host:").width(LABEL_COLUMN),
                // Sized for a full IPv4 address ("255.255.255.255", 15 chars).
                // Without an explicit width these default to Length::Fill and
                // stretch to the window edge, which reads as though a very long
                // value were expected. A longer entry (a hostname, or an
                // "http://" prefix, both of which the client accepts) still
                // works — the field scrolls.
                text_input("192.168.1.64", &state.host)
                    .id(host_input_id())
                    .on_input(Message::HostChanged)
                    .width(FIELD_WIDTH),
            ]
            .spacing(8)
            .align_y(Vertical::Center),
            row![
                text("Password:").width(LABEL_COLUMN),
                // Same width as the host field above it. This is display width
                // only and does not limit what can be typed: silently truncating
                // a device password would make authentication fail with no
                // visible cause.
                text_input("", &state.password)
                    .on_input(Message::PasswordChanged)
                    .secure(true)
                    .width(FIELD_WIDTH),
            ]
            .spacing(8)
            .align_y(Vertical::Center),
            text("Neither host nor password is saved — re-enter both each time you start QuickMon."),
            row![
                // Disabled until a host is entered: with a blank host the
                // request cannot go anywhere, and `State::client` would only
                // report the same thing after the fact.
                button("Connect").on_press_maybe(
                    (!state.host.trim().is_empty()).then_some(Message::TestConnection),
                ),
                match &state.connection {
                    Some(ConnectionStatus::Connected(msg)) =>
                        text(msg).font(BOLD).color(OK_GREEN),
                    Some(ConnectionStatus::Failed(msg)) =>
                        text(msg).font(BOLD).color(FAIL_RED),
                    Some(ConnectionStatus::Testing) => text("Testing…"),
                    None => text(""),
                },
            ]
            .spacing(8)
            .align_y(Vertical::Center),
        ]
        .spacing(6)
        .into()
    } else {
        column![].into()
    };

    // No scrollbar, deliberately. iced 0.14's text_editor has no scrollbar
    // support — the word does not appear in the widget's source — so this is
    // not a styling option that has been missed. It does scroll: it consumes
    // WheelScrolled while the pointer is over it (text_editor.rs:1318) and
    // follows the cursor.
    //
    // The one workaround is to set height(Shrink) so every line renders and
    // wrap this in a scrollable, which yields a real draggable bar. Do not:
    // text_editor still eats the wheel first, and with Shrink its internal
    // offset never moves, so the result is a bar you can drag and a mouse
    // wheel that does nothing. Reviewed and declined; revisit if iced adds
    // scrollbar support upstream.
    let editor = text_editor(&state.source)
        .on_action(Message::SourceEdited)
        .font(iced::Font::MONOSPACE)
        // Tab inserts a tab. The default binding drops it: the insert arm ends
        // with `text.chars().find(|c| !c.is_control())?` (text_editor.rs:1222)
        // and U+0009 is a control character, so the keypress is discarded and
        // nothing happens at all.
        //
        // A real '\t' rather than spaces — cosmic-text lays tabs out at
        // 8-column stops (tab_width defaults to 8), the parser already splits
        // on char::is_whitespace, and it survives a save/open round trip as
        // typed. Only unmodified Tab is captured, leaving Shift+Tab free for
        // an unindent binding later.
        .key_binding(|press| {
            use iced::keyboard::{key, Key};
            let is_plain_tab = matches!(press.key.as_ref(), Key::Named(key::Named::Tab))
                && !press.modifiers.shift();
            let focused = matches!(press.status, text_editor::Status::Focused { .. });
            if is_plain_tab && focused {
                return Some(text_editor::Binding::Insert('\t'));
            }
            text_editor::Binding::from_key_press(press)
        })
        .height(Length::Fill);

    let right: Element<Message> = if state.errors.is_empty() {
        let body = state
            .assembly
            .as_ref()
            .map(|a| listing_text(a, &state.source.text()))
            .unwrap_or_default();
        scrollable(text(body).font(iced::Font::MONOSPACE)).into()
    } else {
        let body = state
            .errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        scrollable(text(body).font(iced::Font::MONOSPACE)).into()
    };

    let memview = column![
        row![
            text("Memory  $"),
            text_input("0400", &state.mem_addr_text)
                .on_input(Message::MemAddrChanged)
                .width(Length::Fixed(90.0)),
            text("Length"),
            text_input("256", &state.mem_len_text)
                .on_input(Message::MemLenChanged)
                .width(Length::Fixed(90.0)),
            button("Read").on_press(Message::MemRead),
        ]
        .spacing(8)
        .align_y(Vertical::Center),
        scrollable(text(state.mem_rows.join("\n")).font(iced::Font::MONOSPACE))
            .height(Length::Fixed(160.0)),
    ]
    .spacing(6);

    column![
    container(column![
        toolbar,
        settings,
        row![
            container(editor)
                .width(Length::FillPortion(1))
                .height(Length::Fill),
            // Framed to match the editor beside it: text_editor draws its own
            // border, a bare scrollable does not, so the two panes read as
            // different kinds of thing without this.
            container(right)
                .width(Length::FillPortion(1))
                .height(Length::Fill)
                .padding(5)
                .style(container::bordered_box),
        ]
        .spacing(8)
        .height(Length::Fill),
        memview,
    ]
    .spacing(8))
    .padding(10)
    .height(Length::Fill),
    // A status bar, not a floating line: full width, its own padding, and
    // the theme's bordered-box surface so it reads as a distinct region
    // pinned to the bottom edge.
    container(match state.status.kind {
        StatusKind::Good => text(&state.status.text).font(BOLD).color(OK_GREEN),
        StatusKind::Plain => text(&state.status.text),
    })
    .width(Length::Fill)
    .padding([6, 10])
    .style(container::bordered_box),
]
.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asm::assemble;
    use crate::net::{Mismatch, NetError, VerifyReport};

    #[test]
    fn summary_reports_a_verified_write() {
        let r = Ok(VerifyReport { address: 0xC000, written: 11, mismatch: None });
        assert_eq!(send_summary(&r), "11 bytes written to $C000 (49152) — verified");
    }

    #[test]
    fn summary_reports_the_offending_offset_on_mismatch() {
        let r = Ok(VerifyReport {
            address: 0xC000,
            written: 11,
            mismatch: Some(Mismatch { offset: 3, expected: 0xD2, actual: 0x00 }),
        });
        assert_eq!(
            send_summary(&r),
            "Verify FAILED at $C003: sent D2, read back 00"
        );
    }

    #[test]
    fn summary_surfaces_device_reported_errors() {
        let r = Err(NetError::Api { errors: vec!["address out of range".into()] });
        assert_eq!(send_summary(&r), "Device reported: address out of range");
    }

    #[test]
    fn summary_surfaces_transport_and_http_errors() {
        assert!(send_summary(&Err(NetError::Transport("connection refused".into())))
            .contains("connection refused"));
        assert!(send_summary(&Err(NetError::Http { status: 403 })).contains("403"));
    }

    #[test]
    fn listing_shows_address_bytes_and_source() {
        let src = "LDA #$08\nSTA $0400\nRTS";
        let a = assemble(src, 0xC000).unwrap();
        let text = listing_text(&a, src);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "C000  A9 08     LDA #$08");
        assert_eq!(lines[1], "C002  8D 00 04  STA $0400");
        assert_eq!(lines[2], "C005  60        RTS");
    }

    #[test]
    fn listing_skips_blank_and_comment_lines() {
        let src = "; header\n\nRTS";
        let a = assemble(src, 0xC000).unwrap();
        assert_eq!(listing_text(&a, src).lines().count(), 1);
    }

    #[test]
    fn listing_of_empty_program_is_empty() {
        let src = "; nothing here";
        let a = assemble(src, 0xC000).unwrap();
        assert_eq!(listing_text(&a, src), "");
    }

    #[test]
    fn opening_a_file_replaces_the_editor_contents_and_remembers_the_directory() {
        let mut state = State::default();
        let path = std::path::PathBuf::from("/tmp/asm/hi.asm");
        // The returned `Task` performs the actual `Config::save()` disk write;
        // it is intentionally dropped (not polled) here so this test never
        // touches the real config directory. The state mutation under test
        // — `last_dir`, editor contents, status — happens synchronously above
        // that, so it is fully verified without any I/O.
        let _ = update(&mut state, Message::FileOpened(Ok((path.clone(), "RTS\n".to_string()))));
        assert_eq!(state.source.text().trim_end(), "RTS");
        assert_eq!(state.config.last_dir, Some(std::path::PathBuf::from("/tmp/asm")));
        assert!(state.status.text.contains("hi.asm"), "status was: {}", state.status.text);
    }

    #[test]
    fn a_failed_open_leaves_the_editor_untouched() {
        let mut state = State::default();
        let before = state.source.text();
        let _ = update(&mut state, Message::FileOpened(Err("permission denied".to_string())));
        assert_eq!(state.source.text(), before);
        assert!(state.status.text.contains("permission denied"), "status was: {}", state.status.text);
    }

    // --- BLOCKING 1: stale `Assembly` must not survive an edit ---

    #[test]
    fn editing_the_source_after_assembling_clears_the_stale_assembly() {
        let mut state = State::default();
        let _ = update(&mut state, Message::Assemble);
        assert!(state.assembly.is_some(), "precondition: an assembly exists before the edit");

        let action = text_editor::Action::Edit(text_editor::Edit::Insert('x'));
        let _ = update(&mut state, Message::SourceEdited(action));

        assert!(state.assembly.is_none(), "a source edit must invalidate the previous assembly");
        assert!(state.errors.is_empty());
    }

    #[test]
    fn non_editing_source_actions_leave_the_assembly_intact() {
        // `text_editor::Action` has nine variants; only `Edit(_)` changes the
        // buffer. Cursor motion, clicks, drags, and scrolling all route
        // through the same `SourceEdited` message (that's how the cursor
        // moves at all), so this pins that they must NOT clear a valid
        // assembly — otherwise clicking in the editor, or moving the cursor
        // toward a reported error line, would blank the listing/error pane
        // for no reason. Paired with the positive case above so the gate
        // can't be simplified away in either direction.
        let mut state = State::default();
        let _ = update(&mut state, Message::Assemble);
        assert!(state.assembly.is_some(), "precondition: an assembly exists before the action");

        let _ = update(
            &mut state,
            Message::SourceEdited(text_editor::Action::Move(text_editor::Motion::Right)),
        );
        assert!(state.assembly.is_some(), "cursor motion must not invalidate the assembly");

        let _ = update(
            &mut state,
            Message::SourceEdited(text_editor::Action::Click(iced::Point::ORIGIN)),
        );
        assert!(state.assembly.is_some(), "a click must not invalidate the assembly");
    }

    #[test]
    fn changing_the_target_after_assembling_clears_the_stale_assembly() {
        let mut state = State::default();
        let _ = update(&mut state, Message::Assemble);
        assert!(state.assembly.is_some(), "precondition: an assembly exists before the change");

        let _ = update(&mut state, Message::TargetChanged("C100".into()));

        assert!(state.assembly.is_none(), "a target change must invalidate the previous assembly");
        assert!(state.errors.is_empty());
    }

    #[test]
    fn clear_stale_assembly_empties_both_assembly_and_errors() {
        // Direct coverage of the shared helper both message arms call through,
        // independent of the text_editor::Action plumbing exercised above.
        let mut state = State::default();
        let _ = update(&mut state, Message::Assemble);
        state.errors = vec![crate::asm::AsmError::new(1, "stale error")];
        assert!(state.assembly.is_some());

        clear_stale_assembly(&mut state);

        assert!(state.assembly.is_none());
        assert!(state.errors.is_empty());
    }

    // --- BLOCKING 2: the $0000/$0001 hardware guard, and the other Send guards ---

    #[test]
    fn send_guard_rejects_writes_that_touch_the_6510_io_port() {
        // Sound only because `assemble()` (src/asm/encode.rs) already rejects
        // any program where `org + len > 0x10000`; see the comment at the
        // guard in `update()`.
        let mut state = State::default();
        state.host = "1.2.3.4".into();
        state.assembly = Some(assemble("RTS", 0x0001).unwrap());
        let _ = update(&mut state, Message::Send);
        assert!(!state.sending);
        assert!(state.status.text.contains("6510"), "status was: {}", state.status.text);
    }

    #[test]
    fn send_guard_rejects_an_empty_program() {
        // An empty buffer assembles successfully to zero bytes, so
        // `assembly.is_some()` is true and the old guard let it through — the
        // device would answer "0 bytes written — verified", which is accurate
        // and tells the user nothing.
        for src in ["", "; just a comment\n", "\n\n\n"] {
            let mut state = State::default();
            state.host = "1.2.3.4".into();
            state.assembly = Some(assemble(src, 0xC000).unwrap());
            assert!(
                !state.has_bytes_to_send(),
                "{src:?} assembled to zero bytes, so Send must be disabled"
            );

            let task = update(&mut state, Message::Send);
            assert_eq!(task.units(), 0, "{src:?} must not produce a network task");
            assert!(!state.sending);
            assert!(
                state.status.text.contains("empty"),
                "status was: {}",
                state.status.text
            );
        }
    }

    #[test]
    fn send_is_offered_once_there_are_bytes() {
        let mut state = State::default();
        state.host = "1.2.3.4".into();
        assert!(!state.has_bytes_to_send(), "nothing assembled yet");
        state.assembly = Some(assemble("RTS", 0xC000).unwrap());
        assert!(state.has_bytes_to_send(), "RTS is one byte — Send must be offered");
    }

    #[test]
    fn send_guard_rejects_when_nothing_is_assembled() {
        let mut state = State::default();
        state.host = "1.2.3.4".into();
        assert!(state.assembly.is_none());
        let _ = update(&mut state, Message::Send);
        assert!(!state.sending);
        assert!(state.status.text.contains("Nothing assembled"), "status was: {}", state.status.text);
    }

    #[test]
    fn a_successful_connection_closes_the_panel_and_greens_the_status_bar() {
        let mut state = State::default();
        state.show_settings = true;
        let info = crate::net::DeviceInfo {
            product: "C64 Ultimate".into(),
            firmware_version: "1.1.0".into(),
            hostname: "ultimate".into(),
        };
        let _ = update(
            &mut state,
            Message::ConnectionTested(Ok(("0.1".to_string(), info))),
        );
        assert!(!state.show_settings, "a successful connect must close the panel");
        assert!(state.connection.is_none(), "the panel message must not linger");
        assert_eq!(state.status.kind, StatusKind::Good, "status must render as good news");
        assert!(
            state.status.text.contains("C64 Ultimate"),
            "status was: {}",
            state.status.text
        );
    }

    #[test]
    fn a_failed_connection_leaves_the_panel_open_to_be_corrected() {
        let mut state = State::default();
        state.show_settings = true;
        let _ = update(
            &mut state,
            Message::ConnectionTested(Err(NetError::Transport("builder error".into()))),
        );
        assert!(
            state.show_settings,
            "the fix is in this panel — closing it would hide the host field"
        );
        assert_eq!(
            state.connection,
            Some(ConnectionStatus::Failed("Cannot reach the device".into()))
        );
        assert_eq!(state.status.kind, StatusKind::Plain, "a failure is not good news");
    }

    #[test]
    fn a_plain_status_set_after_a_good_one_does_not_stay_green() {
        // The colour travels with the text, so it cannot be left behind by a
        // later assignment that forgets to reset it.
        let mut state = State::default();
        state.status = Status::good("Connected — something");
        assert_eq!(state.status.kind, StatusKind::Good);
        state.status = "Ready".into();
        assert_eq!(state.status.kind, StatusKind::Plain);
    }

    #[test]
    fn assemble_status_carries_both_hex_and_decimal() {
        // $C000 is what the listing shows; 49152 is what gets typed into
        // `SYS <n>` on the C64. Both belong in the status line.
        let mut state = State::default();
        state.source = text_editor::Content::with_text("RTS\n");
        state.target_text = "C000".into();
        let _ = update(&mut state, Message::Assemble);
        assert!(
            state.status.text.contains("$C000") && state.status.text.contains("49152"),
            "status was: {}",
            state.status.text
        );
    }

    #[test]
    fn clear_empties_the_editor_and_both_output_panes() {
        let mut state = State::default();
        state.source = text_editor::Content::with_text("LDA #$08\nRTS\n");
        state.assembly = Some(assemble("RTS", 0xC000).unwrap());
        state.errors = vec![crate::asm::AsmError::new(1, "stale error")];

        let _ = update(&mut state, Message::Clear);

        assert!(state.source.text().trim().is_empty(), "editor must be empty");
        assert!(state.assembly.is_none(), "the listing must be cleared");
        assert!(state.errors.is_empty(), "the error pane must be cleared");
        assert_eq!(state.status.text, "Cleared");
    }

    #[test]
    fn clear_leaves_the_memory_viewer_alone() {
        // The memory pane shows what was read off the device, not anything
        // produced from this buffer, so Clear has no business discarding it.
        let mut state = State::default();
        state.mem_rows = vec!["0400  08 09".into()];
        let _ = update(&mut state, Message::Clear);
        assert_eq!(state.mem_rows, vec!["0400  08 09".to_string()]);
    }

    #[test]
    fn clear_is_only_offered_when_there_is_something_to_clear() {
        let mut state = State::default();
        assert!(!state.has_anything_to_clear(), "a fresh session has nothing to clear");

        state.source = text_editor::Content::with_text("RTS\n");
        assert!(state.has_anything_to_clear(), "editor text counts");

        let mut state = State::default();
        state.assembly = Some(assemble("RTS", 0xC000).unwrap());
        assert!(state.has_anything_to_clear(), "a listing counts even with an empty editor");

        let mut state = State::default();
        state.errors = vec![crate::asm::AsmError::new(1, "boom")];
        assert!(state.has_anything_to_clear(), "errors count even with an empty editor");
    }

    #[test]
    fn clear_then_send_is_refused() {
        // Clear drops the assembly, so the zero-byte guard must catch a Send
        // that follows it rather than shipping the previous program.
        let mut state = State::default();
        state.host = "1.2.3.4".into();
        state.assembly = Some(assemble("RTS", 0xC000).unwrap());
        assert!(state.has_bytes_to_send());

        let _ = update(&mut state, Message::Clear);
        assert!(!state.has_bytes_to_send(), "Send must go dark after Clear");

        let task = update(&mut state, Message::Send);
        assert_eq!(task.units(), 0, "Clear then Send must not reach the device");
    }

    #[test]
    fn boot_opens_settings_so_the_host_can_be_entered() {
        // The host is never persisted, so every session must start by typing
        // one. If this regresses to `false` the first action of every session
        // becomes hunting for the gear icon.
        let (state, _task) = boot();
        assert!(state.show_settings, "settings must be open on launch");
        assert!(state.host.is_empty(), "host must start blank — it is never persisted");
    }

    #[test]
    fn send_guard_rejects_when_host_is_empty() {
        let mut state = State::default();
        state.assembly = Some(assemble("RTS", 0xC000).unwrap());
        assert!(state.host.trim().is_empty());
        let _ = update(&mut state, Message::Send);
        assert!(!state.sending);
        assert!(state.status.text.contains("No host configured"), "status was: {}", state.status.text);
    }

    #[test]
    fn send_sets_the_sending_flag_on_the_success_path() {
        let mut state = State::default();
        state.host = "1.2.3.4".into();
        state.assembly = Some(assemble("RTS", 0xC000).unwrap());
        // The returned `Task` performs the actual network call; it is
        // intentionally dropped without polling, exactly like the
        // `FileOpened`/`FileSaved` tests above — the synchronous state
        // mutation is what's under test here.
        let _ = update(&mut state, Message::Send);
        assert!(state.sending);
        assert!(state.status.text.contains("Sending"), "status was: {}", state.status.text);
    }

    // --- FIX-BEFORE-MERGE 6: the empty-host guard must cover MemRead too ---

    #[test]
    fn mem_read_with_blank_host_sets_status_and_issues_no_request() {
        let mut state = State::default();
        assert!(state.host.trim().is_empty());
        let task = update(&mut state, Message::MemRead);
        // A real request would come back as a `Task::perform(...)`, which
        // reports 1 work unit; on the guard path `update` must return early
        // with `Task::none()` (0 units) so no request is ever issued.
        assert_eq!(task.units(), 0, "blank host must not produce a network task");
        assert!(state.status.text.contains("No host configured"), "status was: {}", state.status.text);
        assert!(state.mem_rows.is_empty());
    }
}
