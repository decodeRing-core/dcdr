use std::future::Future;
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

pub async fn with_spinner<F, T>(message: &str, fut: F) -> T
where
    F: Future<Output = T>,
{
    let bar = ProgressBar::new_spinner();
    if let Ok(style) = ProgressStyle::with_template("{spinner:.cyan} {msg}") {
        bar.set_style(style);
    }
    bar.enable_steady_tick(Duration::from_millis(80));
    bar.set_message(message.to_owned());
    let out = fut.await;
    bar.finish_and_clear();
    out
}
