use futures::{Stream, StreamExt};
use tracing::{debug, info, instrument};

use zbus::{fdo::MonitoringProxy, Connection, MatchRule, MessageStream, MessageType};
mod notification;
use notification::Notification;
mod error;
use error::NotifyError;
#[instrument]
pub async fn listen_to_dbus_notifications() -> Result<impl Stream<Item = Notification>, NotifyError>
{
    info!("initializing dbus connection");
    let connection = Connection::session().await?;
    info!("setting dbus connection to monitor mode");
    let monitor = MonitoringProxy::builder(&connection)
        .destination("org.freedesktop.DBus")?
        .interface("org.freedesktop.DBus.Monitoring")?
        .path("/org/freedesktop/DBus")?
        .build()
        .await?;
    info!("connection is now in monitor mode");
    debug!("creating notifications filtering rule");
    let notify_rule = MatchRule::builder()
        .interface("org.freedesktop.Notifications")?
        .path("/org/freedesktop/Notifications")?
        .msg_type(MessageType::MethodCall)
        .member("Notify")?
        .build();
    debug!(?notify_rule, "finished generating rule");
    info!("listening for notifications");
    let notify_rule = notify_rule.to_string();
    monitor.become_monitor(&[notify_rule.as_str()], 0).await?;

    let stream = MessageStream::from(monitor.connection()).filter_map(move |message| async {
        let notification = message.ok()?.try_into().ok()?;
        debug!(?notification, "adding notification to stream");
        Some(notification)
    });
    Ok(Box::pin(stream))
}
