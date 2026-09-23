use std::io::BufRead;

pub const MAX_WORKER_FRAME_BYTES: usize = 1_048_576;

pub fn read_bounded_frame(reader: &mut impl BufRead) -> Result<Option<String>, String> {
    let mut frame = Vec::new();
    loop {
        let available = reader.fill_buf().map_err(|error| error.to_string())?;
        if available.is_empty() {
            return if frame.is_empty() {
                Ok(None)
            } else {
                Err("stream ended with an unterminated frame".to_owned())
            };
        }

        let newline = available.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(available.len());
        if frame.len() + take > MAX_WORKER_FRAME_BYTES {
            return Err(format!("frame exceeded the {MAX_WORKER_FRAME_BYTES}-byte limit"));
        }
        frame.extend_from_slice(&available[..take]);
        reader.consume(take + usize::from(newline.is_some()));

        if newline.is_some() {
            if frame.last() == Some(&b'\r') {
                frame.pop();
            }
            return String::from_utf8(frame).map(Some).map_err(|error| error.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn bounded_reader_handles_valid_eof_and_oversized_frames() {
        let mut valid = Cursor::new(b"payload\r\n".to_vec());
        assert_eq!(read_bounded_frame(&mut valid).unwrap().as_deref(), Some("payload"));
        assert_eq!(read_bounded_frame(&mut valid).unwrap(), None);

        let mut oversized = Cursor::new(vec![b'x'; MAX_WORKER_FRAME_BYTES + 1]);
        assert!(read_bounded_frame(&mut oversized).is_err());
    }
}
