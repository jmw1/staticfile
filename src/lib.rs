#![crate_id = "staticfile"]
#![deny(missing_doc)]
#![feature(phase)]

//! Static file-serving middleware.

extern crate url;
#[phase(plugin)]
extern crate regex_macros;
extern crate regex;

#[phase(plugin, link)]
extern crate log;
extern crate http;
extern crate iron;
extern crate mount;

use iron::{Request, Response, Middleware, Alloy};
use iron::mixin::{GetUrl, Serve};
use iron::middleware::{Status, Continue, Unwind};
use mount::OriginalUrl;

/// The static file-serving `Middleware`.
#[deriving(Clone)]
pub struct Static {
    root_path: Path
}

#[deriving(Clone)]
struct Favicon {
    favicon_path: Path
}

impl Static {
    /// Create a new instance of `Static` with a given root path.
    ///
    /// This will attempt to serve static files from the given root path.
    /// The path may be relative or absolute. If `Path::new("")` is given,
    /// files will be served from the current directory.
    ///
    /// If a static file exists and can be read from, `enter` will serve it to
    /// the `Response` and `Unwind` the middleware stack with a status of `200`.
    ///
    /// In the case of any error, it will `Continue` through the stack.
    /// If a file should have been read but cannot, due to permissions or
    /// read errors, a different `Middleware` should handle it.
    ///
    /// If the path is '/', it will attempt to serve `index.html`.
    pub fn new(root_path: Path) -> Static {
        Static {
            root_path: root_path
        }
    }

    pub fn favicon(favicon_path: Path) -> Favicon {
        Favicon {
            favicon_path: favicon_path
        }
    }
}

impl Middleware for Static {
    fn enter(&mut self, req: &mut Request, res: &mut Response, alloy: &mut Alloy) -> Status {
        match req.url() {
            Some(path) => {
                // Check for requested file
                match res.serve_file(&self.root_path.join(
                    Path::new(
                        // Coerce to relative path.
                        // We include the slash to ensure that you never have a path like ".index.html"
                        // when you meant "./index.html", see http://is.gd/yz9p0B for an example.
                        "./".to_string().append(path.as_slice())))) {
                    Ok(()) => {
                        debug!("Serving static file at {}.",
                            &self.root_path.join("./".to_string().append(path.as_slice())).display());
                        return Unwind
                    },
                    Err(_) => ()
                }

                // Check for index.html
                let index_path = self.root_path.join(
                    Path::new("./".to_string().append(path.as_slice()))
                        .join("./index.html".to_string()));
                if index_path.is_file() {
                    // Avoid serving as a directory
                    if path.len() > 0 {
                        match path.as_slice().char_at_reverse(path.len()) {
                            '/' => {
                                match res.serve_file(&index_path) {
                                    Ok(()) => {
                                        debug!("Serving static file at {}.",
                                            &index_path.display());
                                        return Unwind
                                    },
                                    Err(err) => {
                                        debug!("Failed while trying to serve index.html: {}", err)
                                        return Continue
                                    }
                                }
                            },
                    // 303:
                            _ => ()
                        }
                    }
                    let redirect_path = match alloy.find::<OriginalUrl>() {
                        Some(&OriginalUrl(ref original_url)) => original_url.clone(),
                        None => path.clone()
                    }.append("/");
                    res.headers.extensions.insert("Location".to_string(), redirect_path.clone());
                    let _ = res.serve(::http::status::SeeOther,
                        format!("Redirecting to {}/", redirect_path).as_slice());
                    return Unwind
                }
            },
            None => ()
        }
        Continue
    }
}

impl Middleware for Favicon {
    fn enter(&mut self, req: &mut Request, res: &mut Response, _alloy: &mut Alloy) -> Status {
        match req.request_uri {
            AbsolutePath(ref path) => {
                if regex!("/favicon$").is_match(path.as_slice()) {
                    match res.serve_file(&self.favicon_path) {
                        Ok(()) => {
                            res.headers.cache_control = Some("max-age=86400".to_str());
                            return Unwind },
                        Err(_) => { return Continue }
                    }
                }
                Continue
            },
            _ => {
                Continue
            }
        }
    }
}
