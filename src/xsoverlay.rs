use std::io;
use std::net::UdpSocket;

use serde::Serialize;
use serde_repr::Serialize_repr;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MessageObject {
    message_type: MessageType,
    index: i32,
    timeout: f32,
    height: f32,
    opacity: f32,
    volume: f32,

    #[serde(serialize_with = "serialize_notification_type")]
    audio_path: NotificationAudio,

    title: String,
    content: String,
    use_base64_icon: bool,

    #[serde(serialize_with = "serialize_notification_type")]
    icon: NotificationIcon,

    source_app: String,
}

impl MessageObject {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }
}

pub struct MessageObjectBuilder {
    source: MessageObject,
}

#[allow(dead_code)]
impl MessageObjectBuilder {
    pub fn new(title: String) -> MessageObjectBuilder {
        MessageObjectBuilder {
            source: MessageObject {
                message_type: MessageType::NotificationPopup,
                index: 0,
                timeout: 1.5,
                height: 175f32,
                opacity: 1.0,
                volume: 0.7,
                audio_path: NotificationAudio::Default,
                title: title,
                content: "".to_owned(),
                use_base64_icon: false,
                icon: NotificationIcon::Default,
                source_app: "xsoverlay_vrc_notifier".to_owned(),
            },
        }
    }

    pub fn build(self) -> MessageObject {
        MessageObject { ..self.source }
    }

    pub fn set_content(mut self, content: String) -> Self {
        self.source.content = content;
        self
    }

    pub fn set_audio(mut self, audio: NotificationAudio) -> Self {
        self.source.audio_path = audio;
        self
    }

    pub fn set_icon(mut self, icon: NotificationIcon, is_base64: bool) -> Self {
        self.source.icon = icon;
        self.source.use_base64_icon = is_base64;
        self
    }

    pub fn set_timeout(mut self, timeout: f32) -> Self {
        self.source.timeout = timeout;
        self
    }
}

#[derive(Serialize_repr, Debug)]
#[repr(u8)]
#[allow(dead_code)]
pub enum MessageType {
    NotificationPopup = 1,
    MediaPlayerInformation = 2,
}

#[derive(Serialize, Debug)]
#[allow(dead_code)]
pub enum NotificationType {
    Default,
    Error,
    Warning,
    Custom(String),
}

pub type NotificationAudio = NotificationType;
pub type NotificationIcon = NotificationType;

fn serialize_notification_type<S>(x: &NotificationType, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(match x {
        NotificationType::Default => "default",
        NotificationType::Warning => "warning",
        NotificationType::Error => "error",
        NotificationType::Custom(v) => v,
    })
}

pub struct NotificationClient {
    socket: UdpSocket,
    endpoint: String,
}

impl NotificationClient {
    pub fn new() -> Result<NotificationClient, io::Error> {
        Self::new_with_endpoint("127.0.0.1", 42069)
    }

    pub fn new_with_endpoint(host: &str, port: i32) -> Result<NotificationClient, io::Error> {
        let socket = UdpSocket::bind("127.0.0.1:0")?;
        Ok(NotificationClient {
            socket: socket,
            endpoint: format!("{}:{}", host, port),
        })
    }

    pub fn send_message(&self, message: &MessageObject) -> Result<(), SendMessageError> {
        let json = message.to_json()?;
        self.socket.send_to(json.as_bytes(), &self.endpoint)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum SendMessageError {
    JsonError(serde_json::Error),
    SendError(io::Error),
}

impl From<serde_json::Error> for SendMessageError {
    fn from(err: serde_json::Error) -> SendMessageError {
        SendMessageError::JsonError(err)
    }
}

impl From<io::Error> for SendMessageError {
    fn from(err: io::Error) -> SendMessageError {
        SendMessageError::SendError(err)
    }
}
