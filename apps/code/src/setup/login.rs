//! Logging in through the browser.
//!
//! The token comes back to a loopback listener rather than through a
//! paste: the port is bound before the browser opens, so the callback has
//! somewhere to land and nothing has to be copied by hand.

use anyhow::{Result, bail};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

/// Room for the callback's request line and headers.
const REQUEST_BYTES: usize = 4096;

const PAGE: &str = "HTTP/1.1 200 OK\r\n\
    Content-Type: text/html\r\n\
    Connection: close\r\n\r\n\
    <html><body><h3>Logged in. You can close this tab.</h3></body></html>";

/// Open the browser and wait for the callback to bring a token back.
pub async fn token(cloud: &str) -> Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    browser(&format!(
        "{cloud}/auth/google?client=terminal&port={port}&scope=llm"
    ))?;

    let (mut stream, _) = listener.accept().await?;
    let mut buf = vec![0u8; REQUEST_BYTES];
    let read = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..read]).into_owned();

    let Some(token) = parse(&request) else {
        bail!("the callback carried no token — the login did not finish");
    };
    stream.write_all(PAGE.as_bytes()).await?;
    stream.shutdown().await?;
    Ok(token)
}

/// The `token` query parameter of the callback's request line.
fn parse(request: &str) -> Option<String> {
    request
        .lines()
        .next()?
        .split_whitespace()
        .nth(1)?
        .split('?')
        .nth(1)?
        .split('&')
        .find_map(|pair| pair.strip_prefix("token=").map(str::to_owned))
}

fn browser(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let opened = std::process::Command::new("open").arg(url).status();
    #[cfg(target_os = "linux")]
    let opened = std::process::Command::new("xdg-open").arg(url).status();
    #[cfg(target_os = "windows")]
    let opened = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .status();

    match opened {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => bail!("the browser exited with {status} — open {url} by hand"),
        Err(e) => bail!("could not open a browser ({e}) — open {url} by hand"),
    }
}
