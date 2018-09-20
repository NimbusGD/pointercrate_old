use actix_web::{
    http::{Method, StatusCode},
    ResponseError,
};

#[derive(Debug, Fail)]
pub enum PointercrateError {
    #[fail(display = "The browser (or proxy) sent a request that this server could not understand.")]
    BadRequest,

    #[fail(display = "The value for the header {} could not be processed", header)]
    InvalidHeaderValue { header: &'static str },

    #[fail(
        display = "The server could not verify that you are authorized to access the URL requested. You either supplied the wrong credentials (e.g. a bad password) or your browser doesn't understand how to supply the credentials required."
    )]
    Unauthorized,

    #[fail(
        display = "You don't have the permission to access the requested resource. It is either read-protected or not readable by the server."
    )]
    Forbidden,

    #[fail(display = "You are banned from submitting records to the demonlist!")]
    BannedFromSubmissions,

    #[fail(
        display = "The requested URL was not found on the server. If you entered the URL manually please check your spelling and try again."
    )]
    NotFound,

    #[fail(display = "No '{}' identified by '{}' found!", model, identified_by)]
    ModelNotFound { model: &'static str, identified_by: String },

    #[fail(display = "The method is not allowed for the requested URL.")]
    MethodNotAllowed { allowed_methods: Vec<Method> },

    #[fail(
        display = "A conflict happened while processing the request. The resource might have been modified while the request was being processed."
    )]
    Conflict,

    #[fail(display = "A request with this methods requires a valid 'Content-Length' header")]
    LengthRequired,

    #[fail(display = "The precondition on the request for the URL failed positive evaluation")]
    PreconditionFailed,

    #[fail(display = "The data value transmitted exceeds the capacity limit.")]
    PayloadTooLarge,

    #[fail(
        display = "The server does not support the media type transmitted in the request/no media type was specified. Expected one of: {:?}",
        expected
    )]
    UnsupportedMediaType { expected: Vec<&'static str> },

    #[fail(display = "The request was well-formed but was unable to be followed due to semeantic errors.")]
    UnprocessableEntity,

    #[fail(display = "This request is required to be conditional; try using \"If-Match\"")]
    PreconditionRequired,

    #[fail(
        display = "The server encountered an internal error and was unable to complete your requests. Either the server is overloaded or there is an error in the application. Please notify a server administrator and have them look at the server logs!"
    )]
    InternalServerError,

    #[fail(
        display = "Internally, an invalid database access has been made. Please notify a server administrator and have them look at the server logs!"
    )]
    DatabaseError,

    #[fail(display = "Failed to retrieve connection to the database. The server might be temporarily overloaded.")]
    DatabaseConnectionError,
}

impl PointercrateError {
    pub fn error_code(&self) -> u16 {
        match self {
            PointercrateError::BadRequest => 40000,
            PointercrateError::InvalidHeaderValue { .. } => 40002,
            PointercrateError::Unauthorized => 40100,
            PointercrateError::Forbidden => 40300,
            PointercrateError::BannedFromSubmissions => 40304,
            PointercrateError::NotFound => 40400,
            PointercrateError::ModelNotFound { .. } => 40401,
            PointercrateError::MethodNotAllowed { .. } => 40500,
            PointercrateError::Conflict => 40900,
            PointercrateError::LengthRequired => 41100,
            PointercrateError::PreconditionFailed => 41200,
            PointercrateError::PayloadTooLarge => 41300,
            PointercrateError::UnsupportedMediaType { .. } => 41500,
            PointercrateError::UnprocessableEntity => 42200,
            PointercrateError::PreconditionRequired => 42800,
            PointercrateError::InternalServerError => 50000,
            PointercrateError::DatabaseError => 50003,
            PointercrateError::DatabaseConnectionError => 50005,
            //_ => unimplemented!(),
        }
    }

    pub fn status_code(&self) -> StatusCode {
        let error_code = self.error_code();
        let status_code = error_code / 100;

        StatusCode::from_u16(status_code).unwrap()
    }
}

impl ResponseError for PointercrateError {
    // TODO: impl
}
