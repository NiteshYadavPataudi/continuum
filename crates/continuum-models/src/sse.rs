use continuum_core::{model::ModelError, CancellationToken};
use futures::{stream::BoxStream, StreamExt};

pub fn parse_sse(
    response: reqwest::Response,
    cancel: CancellationToken,
) -> BoxStream<'static, Result<serde_json::Value, ModelError>> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Result<serde_json::Value, ModelError>>();

    tokio::spawn(async move {
        let mut byte_stream = response.bytes_stream();
        let mut buf = Vec::new();
        let mut data = String::new();

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    let _ = tx.send(Err(ModelError::Cancelled));
                    break;
                }
                chunk = byte_stream.next() => {
                    match chunk {
                        None => {
                            if !data.is_empty() {
                                dispatch(&data, &tx);
                            }
                            break;
                        }
                        Some(Err(e)) => {
                            let _ = tx.send(Err(ModelError::Network(e.to_string())));
                            break;
                        }
                        Some(Ok(bytes)) => {
                            buf.extend_from_slice(&bytes);
                            while let Some(nl) = buf.iter().position(|&b| b == b'\n') {
                                let line: Vec<u8> = buf.drain(..=nl).collect();
                                let line = String::from_utf8_lossy(&line);
                                let line = line.trim();

                                if line.is_empty() {
                                    if !data.is_empty() {
                                        dispatch(&data, &tx);
                                        data.clear();
                                    }
                                } else if let Some(s) = line.strip_prefix("data:") {
                                    data.push_str(s.trim());
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    futures::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    })
    .boxed()
}

fn dispatch(
    data: &str,
    tx: &tokio::sync::mpsc::UnboundedSender<Result<serde_json::Value, ModelError>>,
) {
    match serde_json::from_str(data) {
        Ok(val) => {
            let _ = tx.send(Ok(val));
        }
        Err(e) => {
            let _ = tx.send(Err(ModelError::Malformed(e.to_string())));
        }
    }
}
