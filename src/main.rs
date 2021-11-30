mod reader;
mod vrc;
mod xsoverlay;

use chrono::{DateTime, Duration, Utc};

use crate::reader::{LogLineProcessor, VrChatLogProcessor};
use crate::vrc::log::LogLine;
use crate::xsoverlay::{MessageObjectBuilder, NotificationClient};

struct VrcToXsOverlayNotifier<C>
where
    C: CurrentTimeProvider,
{
    client: xsoverlay::NotificationClient,
    // The last time of OnJoinedRoom or OnLeftRoom detected.
    // At the end of DST, the time provided from log file may be ambiguous.
    // so this field must be assigned with current system time.
    // This field is used to determine whether the join or left event is not caused by moving world.
    notifiable_since: Option<DateTime<Utc>>,
    current_time_provider: C,
}

impl<C: CurrentTimeProvider> VrcToXsOverlayNotifier<C> {
    fn new(client: xsoverlay::NotificationClient, current_time_provider: C) -> Self {
        VrcToXsOverlayNotifier {
            client,
            notifiable_since: None,
            current_time_provider,
        }
    }

    fn to_notification_object(&self, line: vrc::log::LogLine) -> Option<xsoverlay::MessageObject> {
        if let Some(notifiable_since) = self.notifiable_since {
            if self.current_time_provider.current_time() < notifiable_since {
                return None;
            }
        }

        let title = match line.event? {
            vrc::Event::OnPlayerJoined { user_name } => {
                format!("{} joined.", user_name)
            }
            vrc::Event::OnPlayerLeft { user_name } => {
                format!("{} left.", user_name)
            }
            _ => return None,
        };

        Some(MessageObjectBuilder::new(title).set_timeout(1f32).build())
    }
}

trait CurrentTimeProvider {
    fn current_time(&self) -> DateTime<Utc>;
}

struct DefaultCurrentTimeProvider {}

impl CurrentTimeProvider for DefaultCurrentTimeProvider {
    fn current_time(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

impl<C: CurrentTimeProvider> LogLineProcessor for VrcToXsOverlayNotifier<C> {
    fn process_line(&mut self, line: LogLine, is_first: bool) {
        if is_first {
            // do not send any notification.
            return;
        }

        if let Some(event) = &line.event {
            match event {
                vrc::Event::OnJoinedRoom | vrc::Event::OnLeftRoom => {
                    // store the time that sending notification starts.
                    self.notifiable_since =
                        Some(self.current_time_provider.current_time() + Duration::seconds(5));
                }
                _ => (),
            }
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

    let mut notifier = VrcToXsOverlayNotifier::new(client, DefaultCurrentTimeProvider {});
    let mut processor = VrChatLogProcessor::new(vrc::log::get_log_dir_path(), &mut notifier);

    loop {
        match processor.process_log() {
            Ok(()) => (),
            Err(e) => println!("{}", e),
        };
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
