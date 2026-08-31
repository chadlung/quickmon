use crate::net::{DeviceInfo, NetError};

/// The outcome of a connection attempt, in the form the settings panel needs to
/// render it: a message plus enough information to colour it.
///
/// This is an enum rather than a plain `String` because success and failure are
/// styled differently (bold green vs bold red), and deciding that from the text
/// itself — by matching on a prefix — would break the moment the copy changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// A request is in flight.
    Testing,
    /// Handshake succeeded. Carries the device detail line.
    Connected(String),
    /// The attempt failed. Carries what to show the user.
    Failed(String),
}

pub fn connection_summary(result: &Result<(String, DeviceInfo), NetError>) -> ConnectionStatus {
    match result {
        // A device that identifies itself is the only thing reported as
        // connected. The JSON decoding upstream is permissive — absent fields
        // become empty strings rather than an error — so a host that answers
        // 200 with `{}`, or with an error in a shape we do not recognise,
        // otherwise arrives here looking like a success and renders as a bold
        // green "Connected — , firmware  (API )". That blank line is the tell,
        // and it is the wrong colour: nothing has confirmed a C64 Ultimate is
        // on the other end. Treat an unidentified device as a failure.
        Ok((_, info))
            if info.product.trim().is_empty() || info.firmware_version.trim().is_empty() =>
        {
            ConnectionStatus::Failed("Device did not identify itself — check the address".into())
        }

        Ok((version, info)) => ConnectionStatus::Connected(format!(
            "Connected — {}, firmware {} (API {})",
            info.product, info.firmware_version, version
        )),

        // A transport failure means the address went nowhere: unroutable,
        // unreachable, or not a valid address at all. reqwest's own text for
        // these is noise to anyone who is not debugging reqwest ("builder
        // error", "error sending request for url (...)"), so it is dropped
        // entirely. The user's next action is the same in every case: check
        // the address.
        Err(NetError::Transport(_)) => ConnectionStatus::Failed("Cannot reach the device".into()),

        // 403 is deliberately NOT collapsed into the message above. The device
        // answered — it is reachable and working, it just wants a password.
        // Reporting that as "cannot reach the device" would send the user
        // hunting for a network fault that does not exist.
        Err(NetError::Http { status: 403 }) => {
            ConnectionStatus::Failed("Device requires a network password — enter it above".into())
        }

        // Anything else came from the device itself, so its own words are the
        // most useful thing to show.
        Err(e) => ConnectionStatus::Failed(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> DeviceInfo {
        DeviceInfo {
            product: "Commodore 64 Ultimate".into(),
            firmware_version: "3.12".into(),
            hostname: "ultimate".into(),
        }
    }

    #[test]
    fn reports_product_and_firmware_on_success() {
        let r = Ok(("1.0".to_string(), info()));
        assert_eq!(
            connection_summary(&r),
            ConnectionStatus::Connected(
                "Connected — Commodore 64 Ultimate, firmware 3.12 (API 1.0)".into()
            )
        );
    }

    /// A host that answers with a well-formed but empty body decodes into a
    /// `DeviceInfo` of empty strings. Showing that as a green "Connected" is
    /// the one way this screen can tell the user something false.
    #[test]
    fn a_device_that_does_not_identify_itself_is_not_reported_as_connected() {
        let blanks = [
            ("", "3.12"),
            ("Commodore 64 Ultimate", ""),
            ("", ""),
            ("   ", "3.12"),
        ];
        for (product, firmware) in blanks {
            let r = Ok((
                "1.0".to_string(),
                DeviceInfo {
                    product: product.into(),
                    firmware_version: firmware.into(),
                    hostname: String::new(),
                },
            ));
            match connection_summary(&r) {
                ConnectionStatus::Failed(s) => assert!(
                    s.contains("did not identify itself"),
                    "product {product:?} firmware {firmware:?} gave: {s}"
                ),
                other => {
                    panic!("product {product:?} firmware {firmware:?} reported success: {other:?}")
                }
            }
        }
    }

    #[test]
    fn explains_403_as_a_missing_password() {
        let r = Err(NetError::Http { status: 403 });
        match connection_summary(&r) {
            ConnectionStatus::Failed(s) => {
                assert!(s.contains("password"), "summary was: {s}");
                // A reachable device that wants a password must not be
                // described as unreachable.
                assert!(
                    !s.contains("Cannot reach"),
                    "403 means the device answered: {s}"
                );
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn transport_failures_say_only_that_the_device_is_unreachable() {
        // reqwest's wording ("builder error" for a malformed address,
        // "error sending request for url (...)" for an unroutable one) is
        // noise. Neither should reach the user.
        for detail in ["builder error", "error sending request for url (http://x/)"] {
            let r = Err(NetError::Transport(detail.into()));
            assert_eq!(
                connection_summary(&r),
                ConnectionStatus::Failed("Cannot reach the device".into()),
                "transport detail leaked into the message: {detail}"
            );
        }
    }

    #[test]
    fn other_device_errors_keep_their_own_message() {
        let r = Err(NetError::Api {
            errors: vec!["address out of range".into()],
        });
        match connection_summary(&r) {
            ConnectionStatus::Failed(s) => assert!(s.contains("address out of range"), "was: {s}"),
            other => panic!("expected Failed, got {other:?}"),
        }
    }
}
