use std::error::Error;
use std::future::Future;

pub async fn frame<F>(title: &str, fut: F) -> Result<(), Box<dyn Error>>
where
    F: Future<Output = Result<(), Box<dyn Error>>>,
{
    let _ = cliclack::intro(title);
    let result = fut.await;
    match &result {
        Ok(()) => {
            let _ = cliclack::outro("Done");
        }
        Err(e) => {
            let _ = cliclack::outro_cancel(e.to_string());
        }
    }
    result
}
