use colored::Colorize;
use std::net::SocketAddr;
use url::Url;
use y_sweet_core::auth::Authenticator;

pub fn print_server_url(auth: Option<&Authenticator>, url_prefix: Option<&Url>, addr: SocketAddr) {
    let mut url = if let Some(url_prefix) = url_prefix {
        url_prefix.clone()
    } else {
        Url::parse(&format!("ys://{}", addr)).unwrap()
    };

    if let Some(auth) = auth {
        url.set_username(&auth.server_token()).unwrap();
    }

    let token = url.to_string();

    // Note: we need to change the scheme in string form, because changing the scheme of
    // certain Url objects is an error (https://docs.rs/url/latest/url/struct.Url.html#method.set_scheme).
    let token = if let Some(rest) = token.strip_prefix("http://") {
        format!("ys://{}", rest)
    } else if let Some(rest) = token.strip_prefix("https://") {
        format!("yss://{}", rest)
    } else {
        token
    };

    println!("Use the following connection string to connect to y-sweet:");
    println!();
    println!("   {}", token.bright_purple());
    println!();
    println!("For example, the y-sweet examples expect this connection string as an environment variable:");
    println!();
    println!("    cd examples/nextjs");
    println!(
        "    CONNECTION_STRING={} npm run dev",
        token.bright_purple()
    );
    println!();
    if auth.is_some() {
        println!(
            "{} {} {}",
            "****".bright_yellow().bold(),
            "If you are running in production, pass --prod to avoid logging this message."
                .bright_red()
                .bold(),
            "****".bright_yellow().bold(),
        );
        println!();
    }
}

pub fn print_auth_message(auth: &Authenticator) {
    println!("Run y-sweet with the following option to enable authentication:");
    println!();
    println!(
        "   {} {} {}",
        "y-sweet serve".bright_black(),
        "--auth".bright_white().bold(),
        auth.private_key().bright_blue().bold()
    );
    println!();
}
