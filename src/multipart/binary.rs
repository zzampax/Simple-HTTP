fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub fn find_binary(complete_buffer: Vec<u8>, boundary: String) -> Vec<u8> {
    let boundary_end: &[u8; 2] = b"\r\n"; // standard CRLF following boundary
    let double_crlf: &[u8; 4] = b"\r\n\r\n"; // marks end of headers
    let mut file_data: Vec<u8> = Vec::new();

    if let Some(mut start) = find_subslice(&complete_buffer, boundary.as_bytes()) {
        start += boundary.len() + boundary_end.len(); // Skip boundary and CRLF

        while let Some(end) = find_subslice(&complete_buffer[start..], boundary.as_bytes()) {
            let part_start = start;
            let part_end = start + end;

            // Extract headers
            if let Some(headers_end) =
                find_subslice(&complete_buffer[part_start..part_end], double_crlf)
            {
                let headers_end = part_start + headers_end + double_crlf.len();

                // Extract headers as string
                let headers = std::str::from_utf8(
                    &complete_buffer[part_start..headers_end - double_crlf.len()],
                )
                .unwrap();

                // Check if this part is the image part by inspecting the headers
                if headers.contains("Content-Type: image/") {
                    // Extract content
                    let content_start = headers_end;
                    let content_end = part_end - boundary_end.len();

                    if content_start < content_end {
                        file_data
                            .extend_from_slice(&complete_buffer[content_start..content_end]);
                    }
                }
            }

            // Move to the start of the next part
            start = part_end + boundary.len() + boundary_end.len();
        }
    }
    return file_data;
}