//! Shared Hikvision-style point/box request in0..=255 coordinates; y counts from the bottom.
/// Hikvision's coordinate span.
pub const SPAN: u8 = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    ZoomTo,
    ZoomOut,
}

/// A validated position3d request; `start == end` is a point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub start: (u8, u8),
    pub end: (u8, u8),
    pub operation: Operation,
}

impl Request {
    /// Parses the card's JSON. Every coordinate is required and must be an integer in 0..=255;
    /// `operation` defaults to `zoomto`.
    pub fn parse(text: &str) -> Result<Self, String> {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            start_x: u8,
            start_y: u8,
            end_x: u8,
            end_y: u8,
            #[serde(default)]
            operation: Option<String>,
        }
        let w: Wire = serde_json::from_str(text).map_err(|_| "invalid position3d payload")?;
        let operation = match w.operation.as_deref() {
            None | Some("zoomto") => Operation::ZoomTo,
            Some("zoomout") => Operation::ZoomOut,
            Some(_) => return Err("invalid position3d operation".into()),
        };
        Ok(Self {
            start: (w.start_x, w.start_y),
            end: (w.end_x, w.end_y),
            operation,
        })
    }

    pub fn is_point(&self) -> bool {
        self.start == self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn req(start: (u8, u8), end: (u8, u8), operation: Operation) -> Request {
        Request {
            start,
            end,
            operation,
        }
    }
    fn point(x: u8, y: u8) -> Request {
        req((x, y), (x, y), Operation::ZoomTo)
    }
    #[test]
    fn parses_the_cards_payload() {
        let r = Request::parse(
            r#"{"start_x":66,"start_y":189,"end_x":66,"end_y":189,"operation":"zoomto"}"#,
        )
        .unwrap();
        assert_eq!(r, point(66, 189));
        assert!(r.is_point());
        let out = Request::parse(
            r#"{"start_x":0,"start_y":0,"end_x":255,"end_y":255,"operation":"zoomout"}"#,
        )
        .unwrap();
        assert_eq!(out.operation, Operation::ZoomOut);
        assert_eq!(
            Request::parse(r#"{"start_x":1,"start_y":2,"end_x":3,"end_y":4}"#)
                .unwrap()
                .operation,
            Operation::ZoomTo
        );
    }

    #[test]
    fn refuses_anything_else() {
        for bad in [
            "",
            "128,128",
            r#"{"start_x":256,"start_y":0,"end_x":0,"end_y":0}"#,
            r#"{"start_x":-1,"start_y":0,"end_x":0,"end_y":0}"#,
            r#"{"start_x":1.5,"start_y":0,"end_x":0,"end_y":0}"#,
            r#"{"start_x":1,"start_y":0,"end_x":0}"#,
            r#"{"start_x":1,"start_y":0,"end_x":0,"end_y":0,"operation":"spin"}"#,
            r#"{"start_x":1,"start_y":0,"end_x":0,"end_y":0,"speed":3}"#,
        ] {
            assert!(Request::parse(bad).is_err(), "{bad}");
        }
    }
}
