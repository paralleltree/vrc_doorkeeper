mod reader;
mod vrc;
mod xsoverlay;

use crate::reader::{LogLineProcessor, VrChatLogProcessor};
use crate::vrc::log::LogLine;
use crate::xsoverlay::{MessageObjectBuilder, NotificationClient};

struct VrcToXsOverlayNotifier {
    client: xsoverlay::NotificationClient,
    player_name: String,
}

impl VrcToXsOverlayNotifier {
    fn new(client: xsoverlay::NotificationClient) -> Self {
        VrcToXsOverlayNotifier {
            client,
            player_name: "".to_owned(),
        }
    }

    fn to_notification_object(&self, line: vrc::log::LogLine) -> Option<xsoverlay::MessageObject> {
        let title = match line.event? {
            vrc::Event::OnPlayerJoined { user_name } if user_name != self.player_name => {
                format!("{} joined.", user_name)
            }
            vrc::Event::OnPlayerLeft { user_name } if user_name != self.player_name => {
                format!("{} left.", user_name)
            }
            _ => return None,
        };

        Some(MessageObjectBuilder::new(title).set_timeout(1f32).build())
    }
}

impl LogLineProcessor for VrcToXsOverlayNotifier {
    fn process_line(&mut self, line: LogLine, is_first: bool) {
        if let Some(event) = &line.event {
            if let vrc::Event::UserAuthenticated { user_name } = event {
                self.player_name = user_name.to_owned();
            }
        }

        if is_first {
            // do not send any notification.
            return;
        }

        if let Some(message) = self.to_notification_object(line) {
            match self.client.send_message(&message) {
                Ok(()) => (),
                Err(e) => match e {
                    xsoverlay::SendMessageError::JsonError(e) => eprintln!("{}", e),
                    xsoverlay::SendMessageError::SendError(e) => eprintln!("{}", e),
                },
            }
        }
    }
}

fn main() {
    let client = NotificationClient::new().expect("Failed to initialize NotificationClient.");
    let welcome = MessageObjectBuilder::new("VRC Dooker".to_owned())
        .set_content("Join and Leave notification are enabled.".to_owned())
        .set_timeout(2f32)
        .build();
    client
        .send_message(&welcome)
        .expect("Failed to send message.");

    let mut notifier = VrcToXsOverlayNotifier::new(client);
    let mut processor = VrChatLogProcessor::new(vrc::log::get_log_dir_path(), &mut notifier);

    loop {
        match processor.process_log() {
            Ok(()) => (),
            Err(e) => println!("{}", e),
        };
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
