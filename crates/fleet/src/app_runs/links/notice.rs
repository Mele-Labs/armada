//! A refused connection, read from one line of a process's output. **Only a refusal that names a
//! local port is a finding**: a program that cannot reach a sibling says which port it tried, and
//! that is the number the sibling was expected on.

/// The port a line says a connection to this machine was refused on, or none.
pub(crate) fn refused_port(line: &str) -> Option<u16> {
    let refused = line.contains("ECONNREFUSED") || line.contains("onnection refused") || line.contains("http proxy error");
    if !refused {
        return None;
    }
    // `127.0.0.1:8731`, `localhost:8731`, `::1:8731` and curl's `localhost port 8731`.
    for host in ["127.0.0.1", "localhost", "0.0.0.0", "::1"] {
        for (at, _) in line.match_indices(host) {
            let rest = &line[at + host.len()..];
            let Some(rest) = rest.strip_prefix(':').or_else(|| rest.strip_prefix(" port ")) else {
                continue;
            };
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if let Some(port) = digits.parse::<u16>().ok().filter(|port| *port >= 1024) {
                return Some(port);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_refusals_a_dev_server_and_its_tools_print_name_the_port() {
        for line in [
            "Error: connect ECONNREFUSED 127.0.0.1:8731",
            "connect ECONNREFUSED ::1:8731",
            "curl: (7) Failed to connect to localhost port 8731 after 0 ms: Connection refused",
            "[Errno 111] Connection refused while calling http://localhost:8731/api",
            "12:00:01 [vite] http proxy error: /api/x connect ECONNREFUSED 127.0.0.1:8731",
        ] {
            assert_eq!(refused_port(line), Some(8731), "{line}");
        }
    }

    #[test]
    fn a_line_that_is_not_a_refusal_or_names_no_port_is_none() {
        assert_eq!(refused_port("GET http://localhost:8731/ 200"), None);
        assert_eq!(refused_port("AggregateError [ECONNREFUSED]:"), None);
        assert_eq!(refused_port("connect ECONNREFUSED 10.0.0.4:8731"), None);
        assert_eq!(refused_port("connect ECONNREFUSED 127.0.0.1:80"), None);
    }
}
