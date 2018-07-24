use std::fmt;

/// Response status.
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Status {
    /// 100
    Continue,

    /// 101
    SwitchingProtocols,

    /// 103
    Processing,

    /// 200
    Ok,

    /// 201
    Created,

    /// 202
    Accepted,

    /// 203
    NonAuthoritativeInformation,

    /// 204
    NoContent,

    /// 205
    ResetContent,

    /// 206
    PartialContent,

    /// 207
    MultiStatus,

    /// 208
    AlreadyReported,

    /// 226
    ImUsed,

    /// 300
    MultipleChoices,

    /// 301
    MovedPermanently,

    /// 302
    Found,

    /// 303
    SeeOther,

    /// 304
    NotModified,

    /// 305
    UseProxy,

    /// 307
    TemporaryRedirect,

    /// 308
    PermanentRedirect,

    /// 400
    BadRequest,

    /// 401
    Unauthorized,

    /// 402
    PaymentRequired,

    /// 403
    Forbidden,

    /// 404
    NotFound,

    /// 405
    MethodNotAllowed,

    /// 406
    NotAcceptable,

    /// 407
    ProxyAuthenticationRequired,

    /// 408
    RequestTimeout,

    /// 409
    Conflict,

    /// 410
    Gone,

    /// 411
    LengthRequired,

    /// 412
    PreconditionFailed,

    /// 413
    PayloadTooLarge,

    /// 414
    UriTooLong,

    /// 415
    UnsupportedMediaType,

    /// 416
    RangeNotSatisfiable,

    /// 417
    ExceptionFailed,

    /// 418
    ImATeapot,

    /// 421
    MisdirectedRequest,

    /// 422
    UnprocessableEntity,

    /// 423
    Locked,

    /// 424
    FailedDependency,

    /// 426
    UpgradeRequired,

    /// 451
    UnavailableForLegalReasons,

    /// 500
    InternalServerError,

    /// 501
    NotImplemented,

    /// 502
    BadGateway,

    /// 503
    ServiceUnavailable,

    /// 504
    GatewayTimeout,

    /// 505
    HttpVersionNotSupported,

    /// 506
    VariantAlsoNegotiates,

    /// 507
    InsufficientStorage,

    /// 508
    LoopDetected,

    /// 509
    BandwidthLimitExceeded,

    /// 510
    NotExtended,
}
impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.code(), self.reason_phrase())
    }
}
impl Status {
    /// Returns the code of the status.
    pub fn code(self) -> u16 {
        match self {
            Status::Continue => 100,
            Status::SwitchingProtocols => 101,
            Status::Processing => 102,
            Status::Ok => 200,
            Status::Created => 201,
            Status::Accepted => 202,
            Status::NonAuthoritativeInformation => 203,
            Status::NoContent => 204,
            Status::ResetContent => 205,
            Status::PartialContent => 206,
            Status::MultiStatus => 207,
            Status::AlreadyReported => 208,
            Status::ImUsed => 226,
            Status::MultipleChoices => 300,
            Status::MovedPermanently => 301,
            Status::Found => 302,
            Status::SeeOther => 303,
            Status::NotModified => 304,
            Status::UseProxy => 305,
            Status::TemporaryRedirect => 307,
            Status::PermanentRedirect => 308,
            Status::BadRequest => 400,
            Status::Unauthorized => 401,
            Status::PaymentRequired => 402,
            Status::Forbidden => 403,
            Status::NotFound => 404,
            Status::MethodNotAllowed => 405,
            Status::NotAcceptable => 406,
            Status::ProxyAuthenticationRequired => 407,
            Status::RequestTimeout => 408,
            Status::Conflict => 409,
            Status::Gone => 410,
            Status::LengthRequired => 411,
            Status::PreconditionFailed => 412,
            Status::PayloadTooLarge => 413,
            Status::UriTooLong => 414,
            Status::UnsupportedMediaType => 415,
            Status::RangeNotSatisfiable => 416,
            Status::ExceptionFailed => 417,
            Status::ImATeapot => 418,
            Status::MisdirectedRequest => 421,
            Status::UnprocessableEntity => 422,
            Status::Locked => 423,
            Status::FailedDependency => 424,
            Status::UpgradeRequired => 426,
            Status::UnavailableForLegalReasons => 451,
            Status::InternalServerError => 500,
            Status::NotImplemented => 501,
            Status::BadGateway => 502,
            Status::ServiceUnavailable => 503,
            Status::GatewayTimeout => 504,
            Status::HttpVersionNotSupported => 505,
            Status::VariantAlsoNegotiates => 506,
            Status::InsufficientStorage => 507,
            Status::LoopDetected => 508,
            Status::BandwidthLimitExceeded => 509,
            Status::NotExtended => 510,
        }
    }

    /// Returns the typical reason phrase of the status.
    pub fn reason_phrase(self) -> &'static str {
        match self {
            Status::Continue => "Continue",
            Status::SwitchingProtocols => "Switching Protocols",
            Status::Processing => "Processing",
            Status::Ok => "OK",
            Status::Created => "Created",
            Status::Accepted => "Accepted",
            Status::NonAuthoritativeInformation => "Non-Authoritative Information",
            Status::NoContent => "No Content",
            Status::ResetContent => "Reset Content",
            Status::PartialContent => "Partial Content",
            Status::MultiStatus => "Multi-Status",
            Status::AlreadyReported => "Already Reported",
            Status::ImUsed => "IM Used",
            Status::MultipleChoices => "Multiple Choices",
            Status::MovedPermanently => "Moved Permanently",
            Status::Found => "Found",
            Status::SeeOther => "See Other",
            Status::NotModified => "Not Modified",
            Status::UseProxy => "Use Proxy",
            Status::TemporaryRedirect => "Temporary Redirect",
            Status::PermanentRedirect => "Permanent Redirect",
            Status::BadRequest => "Bad Request",
            Status::Unauthorized => "Unauthorized",
            Status::PaymentRequired => "Payment Required",
            Status::Forbidden => "Forbidden",
            Status::NotFound => "Not Found",
            Status::MethodNotAllowed => "Method Not Allowed",
            Status::NotAcceptable => "Not Acceptable",
            Status::ProxyAuthenticationRequired => "Proxy Authentication Required",
            Status::RequestTimeout => "Request Timeout",
            Status::Conflict => "Conflict",
            Status::Gone => "Gone",
            Status::LengthRequired => "Length Required",
            Status::PreconditionFailed => "Precondition Failed",
            Status::PayloadTooLarge => "Payload Too Large",
            Status::UriTooLong => "URI Too Long",
            Status::UnsupportedMediaType => "Unsupported Media Type",
            Status::RangeNotSatisfiable => "Range Not Satisfiable",
            Status::ExceptionFailed => "Expectation Failed",
            Status::ImATeapot => "I'm a teapot",
            Status::MisdirectedRequest => "Misdirected Request",
            Status::UnprocessableEntity => "Unporcessable Entity",
            Status::Locked => "Locked",
            Status::FailedDependency => "Failed Dependency",
            Status::UpgradeRequired => "Upgrade Required",
            Status::UnavailableForLegalReasons => "Unavailable For Legal Reasons",
            Status::InternalServerError => "Internal Server Error",
            Status::NotImplemented => "Not Implemented",
            Status::BadGateway => "Bad Gateway",
            Status::ServiceUnavailable => "Service Unavailable",
            Status::GatewayTimeout => "Gateway Timeout",
            Status::HttpVersionNotSupported => "HTTP Version Not Supported",
            Status::VariantAlsoNegotiates => "Variant Also Negotiates",
            Status::InsufficientStorage => "Insufficient Storage",
            Status::LoopDetected => "Loop Detected",
            Status::BandwidthLimitExceeded => "Bandwidth Limit Exceeded",
            Status::NotExtended => "Not Extended",
        }
    }
}
