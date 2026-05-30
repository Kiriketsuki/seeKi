use std::sync::Arc;
use std::time::Duration;

use super::UpdateState;
use super::github;

pub fn spawn_update_poller(update: Arc<UpdateState>) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;

        let mut last_channel = None;
        loop {
            let settings = {
                let guard = update.settings.lock().await;
                guard.clone()
            };

            let force = last_channel.is_some_and(|channel| channel != settings.pre_release_channel);
            if let Err(error) = refresh_release(&update, settings.pre_release_channel, force).await
            {
                tracing::warn!(error = %error, "Update poller refresh failed");
            }
            last_channel = Some(settings.pre_release_channel);

            wait_for_next_cycle(update.as_ref(), settings.poll_interval_hours).await;
        }
    });
}

async fn refresh_release(
    update: &Arc<UpdateState>,
    include_prerelease: bool,
    force: bool,
) -> anyhow::Result<()> {
    let result = github::check_latest(&update.cache, include_prerelease, force).await?;
    if result.fetched {
        persist_last_checked(update.as_ref()).await?;
    }
    Ok(())
}

async fn persist_last_checked(update: &UpdateState) -> anyhow::Result<()> {
    let mut settings = update.settings.lock().await;
    settings.last_checked = Some(chrono::Utc::now().to_rfc3339());
    settings.save()
}

async fn wait_for_next_cycle(update: &UpdateState, poll_interval_hours: u8) {
    if poll_interval_hours == 0 {
        update.settings_changed.notified().await;
        return;
    }

    let sleep_duration = Duration::from_secs(u64::from(poll_interval_hours) * 60 * 60);
    tokio::select! {
        _ = tokio::time::sleep(sleep_duration) => {}
        _ = update.settings_changed.notified() => {}
    }
}
