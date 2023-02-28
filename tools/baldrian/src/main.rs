use std::{
    convert::Infallible,
    fmt::Write,
    net::{Ipv4Addr, SocketAddrV4},
};

use clap::{command, Parser};
use color_eyre::eyre::{eyre, Result, WrapErr};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde_json::Value;
use tokio::process::Command;

const ALL_NAOS: &[&str] = &[
    "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31", "32",
];

async fn pepsi(sub_command: &str, naos: impl IntoIterator<Item = &str>) -> String {
    let mut command = Command::new("pepsi");
    command.arg(sub_command);
    for nao_number in naos.into_iter() {
        command.arg(nao_number);
    }
    let output = command.output().await;
    match output {
        Ok(output) => format!(
            "ran pepsi {sub_command} (status: {})\n\nstdout:\n{}\n\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
        Err(error) => format!("failed to execute pepsi: {error:#?}"),
    }
}

async fn index_page() -> Result<String> {
    let aliveness_info = Command::new("pepsi")
        .arg("aliveness")
        .arg("--json")
        .output()
        .await
        .wrap_err("failed to run aliveness query")?;
    let json: Value = serde_json::from_slice(&aliveness_info.stdout)?;
    let robots = json
        .as_object()
        .ok_or(eyre!("aliveness info is not an object"))?;
    let mut output = String::new();
    output += "<html>";
    output += "<h2>Poweroff</h2>";
    output += "<ul>";
    output += "<li><a href=\"poweroff\">All Robots</a></li>";
    for (key, _value) in robots.iter() {
        write!(&mut output, "<li><a href=\"poweroff?{key}\">{key}</a></li>")?;
    }
    output += "</ul>";
    output += "<h2>Reboot</h2>";
    output += "<ul>";
    output += "<li><a href=\"reboot\">All Robots</a></li>";
    for (key, _value) in robots.iter() {
        write!(&mut output, "<li><a href=\"reboot?{key}\">{key}</a></li>")?;
    }
    output += "</ul>";
    output += "</html>";
    Ok(output)
}

async fn serve(request: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match (request.method(), request.uri().path()) {
        (&Method::GET, "/") => Ok(Response::new(Body::from(match index_page().await {
            Ok(html) => html,
            Err(error) => format!("<html>{error:#?}</html>"),
        }))),
        (&Method::GET, "/poweroff") => Ok(Response::new(Body::from(match request.uri().query() {
            Some(query) => pepsi("poweroff", query.split(',')).await,
            None => pepsi("poweroff", ALL_NAOS.iter().copied()).await,
        }))),
        (&Method::GET, "/reboot") => Ok(Response::new(Body::from(match request.uri().query() {
            Some(query) => pepsi("reboot", query.split(',')).await,
            None => pepsi("reboot", ALL_NAOS.iter().copied()).await,
        }))),

        // Return the 404 Not Found for other routes.
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

/// Baldrian, puts NAOs to sleep
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Arguments {
    /// port to serve baldrian
    #[arg(long, default_value = "3000")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let arguments = Arguments::parse();
    let local_address = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, arguments.port);
    println!("Listening on http://{local_address}");
    let server = Server::bind(&local_address.into()).serve(make_service_fn(|_connection| async {
        Ok::<_, Infallible>(service_fn(serve))
    }));
    server.await?;
    Ok(())
}
