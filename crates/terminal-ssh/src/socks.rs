//! SOCKS5 CONNECT / 无认证；域名原样交给 SSH 服务器解析。
use std::{
    io,
    net::{Ipv4Addr, Ipv6Addr},
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub(crate) async fn reply<S: AsyncWrite + Unpin>(stream: &mut S, code: u8) -> io::Result<()> {
    stream.write_all(&[5, code, 0, 1, 0, 0, 0, 0, 0, 0]).await
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Invalid SOCKS5 request")
}
pub(crate) async fn request<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
) -> io::Result<(String, u16)> {
    if stream.read_u8().await? != 5 {
        return Err(invalid());
    }
    let count = stream.read_u8().await? as usize;
    let mut methods = vec![0; count];
    stream.read_exact(&mut methods).await?;
    if !methods.contains(&0) {
        stream.write_all(&[5, 255]).await?;
        return Err(invalid());
    }
    stream.write_all(&[5, 0]).await?;
    let mut header = [0; 4];
    stream.read_exact(&mut header).await?;
    if header[0] != 5 || header[2] != 0 {
        reply(stream, 1).await?;
        return Err(invalid());
    }
    if header[1] != 1 {
        reply(stream, 7).await?;
        return Err(invalid());
    }
    let host = match header[3] {
        1 => {
            let mut bytes = [0; 4];
            stream.read_exact(&mut bytes).await?;
            Ipv4Addr::from(bytes).to_string()
        }
        4 => {
            let mut bytes = [0; 16];
            stream.read_exact(&mut bytes).await?;
            Ipv6Addr::from(bytes).to_string()
        }
        3 => {
            let count = stream.read_u8().await? as usize;
            let mut bytes = vec![0; count];
            stream.read_exact(&mut bytes).await?;
            let host = String::from_utf8(bytes).map_err(|_| invalid())?;
            if !host.is_ascii() || !terminal_core::valid_forward_target(&host) {
                reply(stream, 8).await?;
                return Err(invalid());
            }
            host
        }
        _ => {
            reply(stream, 8).await?;
            return Err(invalid());
        }
    };
    let port = stream.read_u16().await?;
    if port == 0 {
        reply(stream, 1).await?;
        return Err(invalid());
    }
    Ok((host, port))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn fragmented_domain_is_preserved_for_remote_dns() {
        let (mut client, mut server) = tokio::io::duplex(64);
        let send = async {
            for byte in [
                5, 1, 0, 5, 1, 0, 3, 7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 1, 187,
            ] {
                client.write_all(&[byte]).await.unwrap();
            }
            let mut method = [0; 2];
            client.read_exact(&mut method).await.unwrap();
            assert_eq!(method, [5, 0]);
        };
        let (_, target) = tokio::join!(send, request(&mut server));
        assert_eq!(target.unwrap(), ("example".into(), 443));
    }
    #[tokio::test]
    async fn udp_associate_is_explicitly_rejected() {
        let (mut client, mut server) = tokio::io::duplex(64);
        client.write_all(&[5, 1, 0, 5, 3, 0, 1]).await.unwrap();
        assert!(request(&mut server).await.is_err());
        let mut response = [0; 12];
        client.read_exact(&mut response).await.unwrap();
        assert_eq!(response[3], 7);
    }
}
