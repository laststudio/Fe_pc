use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const CAP_WIDGET_JS: &str = include_str!("cap-widget-0.1.56.min.js");

pub struct CaptchaServer {
    port: u16,
    rx: Receiver<String>,
    shutdown: Arc<AtomicBool>,
}

impl CaptchaServer {
    pub fn start() -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("绑定本地验证端口失败: {e}"))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("获取本地端口失败: {e}"))?
            .port();

        let (tx, rx) = channel();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        listener.set_nonblocking(false).ok();

        thread::spawn(move || {
            for stream_res in listener.incoming() {
                if shutdown_clone.load(Ordering::Relaxed) {
                    break;
                }

                let stream = match stream_res {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let tx_cl = tx.clone();
                let shutdown_cl = shutdown_clone.clone();
                thread::spawn(move || {
                    handle_client(stream, tx_cl, shutdown_cl);
                });
            }
        });

        Ok(Self {
            port,
            rx,
            shutdown,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn try_recv(&self) -> Option<String> {
        self.rx.try_recv().ok()
    }
}

impl Drop for CaptchaServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        // 主动向本地端口发起一次轻量短连接，唤醒阻塞在 incoming().next() 的后台线程使其安全退出
        let _ = std::net::TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], self.port)),
            std::time::Duration::from_millis(150),
        );
    }
}

fn handle_client(mut stream: TcpStream, tx: Sender<String>, shutdown: Arc<AtomicBool>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let mut buf = Vec::new();
    let mut temp = [0u8; 4096];
    let mut header_end = None;
    let mut content_length = 0;

    loop {
        let n = match stream.read(&mut temp) {
            Ok(n) if n > 0 => n,
            _ => break,
        };
        buf.extend_from_slice(&temp[..n]);

        if header_end.is_none() {
            if let Some(pos) = find_subsequence(&buf, b"\r\n\r\n") {
                header_end = Some(pos + 4);
                let headers_str = String::from_utf8_lossy(&buf[..pos]);
                for line in headers_str.lines() {
                    if let Some(val) = line.to_lowercase().strip_prefix("content-length:") {
                        content_length = val.trim().parse::<usize>().unwrap_or(0);
                        break;
                    }
                }
            } else if let Some(pos) = find_subsequence(&buf, b"\n\n") {
                header_end = Some(pos + 2);
                let headers_str = String::from_utf8_lossy(&buf[..pos]);
                for line in headers_str.lines() {
                    if let Some(val) = line.to_lowercase().strip_prefix("content-length:") {
                        content_length = val.trim().parse::<usize>().unwrap_or(0);
                        break;
                    }
                }
            }
        }

        if let Some(hend) = header_end
            && buf.len() >= hend + content_length
        {
            break;
        }
    }

    if buf.is_empty() {
        return;
    }

    let hend = header_end.unwrap_or(buf.len());
    let req_headers = String::from_utf8_lossy(&buf[..hend]);
    let req_line = req_headers.lines().next().unwrap_or("");
    let body = if hend < buf.len() { &buf[hend..] } else { &[] };

    if req_line.starts_with("OPTIONS ") {
        let resp = "HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: *\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes());
        let _ = stream.flush();
    } else if req_line.starts_with("GET /submit_token") {
        if let Some(pos) = req_line.find("token=") {
            let after = &req_line[pos + 6..];
            let token_raw = after
                .split([' ', '&', '\r', '\n'])
                .next()
                .unwrap_or("");
            let token = percent_decode(token_raw);
            if !token.is_empty() {
                let _ = tx.send(token);
                shutdown.store(true, Ordering::Relaxed);
            }
        }
        let body_str = "OK";
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body_str.len(),
            body_str
        );
        let _ = stream.write_all(resp.as_bytes());
        let _ = stream.flush();
    } else if req_line.starts_with("POST /cap_proxy/challenge") {
        let res = ureq::post("https://lastudio.cc/cap/f0d2ec6d51/challenge")
            .timeout(Duration::from_secs(10))
            .set("Origin", "https://lastudio.cc")
            .set("Referer", "https://lastudio.cc/")
            .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .call();

        let (status_code, body_str) = match res {
            Ok(r) => (200, r.into_string().unwrap_or_default()),
            Err(ureq::Error::Status(c, r)) => (c, r.into_string().unwrap_or_default()),
            Err(e) => (500, format!(r#"{{"error":"{}"}}"#, e)),
        };

        let resp = format!(
            "HTTP/1.1 {status_code} OK\r\nContent-Type: application/json; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body_str.len(),
            body_str
        );
        let _ = stream.write_all(resp.as_bytes());
        let _ = stream.flush();
    } else if req_line.starts_with("POST /cap_proxy/redeem") {
        let res = ureq::post("https://lastudio.cc/cap/f0d2ec6d51/redeem")
            .timeout(Duration::from_secs(10))
            .set("Origin", "https://lastudio.cc")
            .set("Referer", "https://lastudio.cc/")
            .set("Content-Type", "application/json")
            .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .send_bytes(body);

        let (status_code, body_str) = match res {
            Ok(r) => (200, r.into_string().unwrap_or_default()),
            Err(ureq::Error::Status(c, r)) => (c, r.into_string().unwrap_or_default()),
            Err(e) => (500, format!(r#"{{"error":"{}"}}"#, e)),
        };

        let resp = format!(
            "HTTP/1.1 {status_code} OK\r\nContent-Type: application/json; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body_str.len(),
            body_str
        );
        let _ = stream.write_all(resp.as_bytes());
        let _ = stream.flush();
    } else if req_line.starts_with("GET /favicon.ico") {
        let resp = "HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes());
        let _ = stream.flush();
    } else {
        let html = build_captcha_html();
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            html.len(),
            html
        );
        let _ = stream.write_all(resp.as_bytes());
        let _ = stream.flush();
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

#[inline(never)]
fn percent_decode(input: &str) -> String {
    crate::themida_tiger_white!({
        let mut bytes = Vec::new();
        let mut chars = input.bytes();
        while let Some(b) = chars.next() {
            if b == b'%' {
                let h1 = chars.next();
                let h2 = chars.next();
                if let (Some(h1), Some(h2)) = (h1, h2) {
                    let hex_str = format!("{}{}", h1 as char, h2 as char);
                    if let Ok(val) = u8::from_str_radix(&hex_str, 16) {
                        bytes.push(val);
                        continue;
                    }
                }
            } else if b == b'+' {
                bytes.push(b' ');
                continue;
            }
            bytes.push(b);
        }
        String::from_utf8_lossy(&bytes).to_string()
    })
}

fn build_captcha_html() -> String {
    format!(
        r#"<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>LA Studio 人机安全验证 - Fe PC</title>
  <style>
    body {{
      background: #181818;
      color: #f0f0f0;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
      display: flex;
      justify-content: center;
      align-items: center;
      min-height: 100vh;
      margin: 0;
      padding: 16px;
      box-sizing: border-box;
    }}
    .card {{
      background: #232323;
      border: 1px solid #383838;
      border-radius: 14px;
      padding: 32px 36px;
      max-width: 440px;
      width: 100%;
      box-shadow: 0 16px 36px rgba(0,0,0,0.6);
      text-align: center;
    }}
    .logo {{
      font-size: 36px;
      margin-bottom: 8px;
    }}
    h2 {{
      margin: 0 0 8px 0;
      color: #40c4ff;
      font-size: 20px;
      font-weight: 600;
    }}
    p {{
      color: #a0a0a0;
      font-size: 13.5px;
      line-height: 1.5;
      margin: 0 0 24px 0;
    }}
    .widget-box {{
      display: flex;
      justify-content: center;
      margin-bottom: 16px;
      min-height: 60px;
    }}
    .status-box {{
      display: none;
      padding: 12px;
      border-radius: 8px;
      font-size: 14px;
      font-weight: bold;
      margin-top: 16px;
      line-height: 1.4;
    }}
    .success {{
      display: block;
      background: rgba(76, 175, 80, 0.15);
      border: 1px solid #4caf50;
      color: #4caf50;
    }}
    .error {{
      display: block;
      background: rgba(244, 67, 54, 0.15);
      border: 1px solid #f44336;
      color: #f44336;
    }}
  </style>
  <script>
{}
  </script>
</head>
<body>
  <div class="card">
    <div class="logo">🛡️</div>
    <h2>LA Studio 人机安全验证</h2>
    <p>为确保当前登录为您本人操作，请点击下方进行安全验证。验证完成后桌面客户端将自动完成登录。</p>
    <div class="widget-box">
      <cap-widget data-cap-api-endpoint="/cap_proxy/"
                  data-cap-i18n-initial-state="点击开始验证"
                  data-cap-i18n-verifying-label="正在安全计算验证..."
                  data-cap-i18n-solved-label="✔ 验证成功"></cap-widget>
    </div>
    <div id="status" class="status-box"></div>
  </div>
  <script>
    customElements.whenDefined('cap-widget').then(() => {{
      const w = document.querySelector('cap-widget');
      w.addEventListener('solve', e => {{
        const token = e.detail.token;
        const statusBox = document.getElementById('status');
        statusBox.className = 'status-box success';
        statusBox.innerHTML = '✔ 验证成功！桌面客户端正在自动登录，您可以关闭本窗口。';
        fetch('/submit_token?token=' + encodeURIComponent(token))
          .catch(() => {{}});
        setTimeout(() => {{ window.close(); }}, 3000);
      }});
      w.addEventListener('error', e => {{
        const statusBox = document.getElementById('status');
        statusBox.className = 'status-box error';
        statusBox.innerText = '验证遇到问题: ' + (e.detail.message || '请刷新页面重试');
      }});
    }});
  </script>
</body>
</html>"#,
        CAP_WIDGET_JS
    )
}
