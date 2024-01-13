import Foundation

extension Data {
    /// Same as ``Data(base64Encoded:)``, but adds padding automatically
    /// (if missing, instead of returning `nil`).
    fileprivate static func fromBase64(_ encoded: String) -> Data? {
        // Prefixes padding-character(s) (if needed).
        var encoded = encoded;
        let remainder = encoded.count % 4
        if remainder > 0 {
            encoded = encoded.padding(
                toLength: encoded.count + 4 - remainder,
                withPad: "=", startingAt: 0);
        }

        // Finally, decode.
        return Data(base64Encoded: encoded);
    }
}

extension String {
    fileprivate static func fromBase64(_ encoded: String) -> String? {
        if let data = Data.fromBase64(encoded) {
            return String(data: data, encoding: .utf8)
        }
        return nil;
    }
}

func precompileIncludeStr(_ path: String) -> String {
    var content: String = ""
    switch (path) {
        // <precompile-content-str>
        default:
            fatalError("Error: include file not found: \(path)")
    }

    return String.fromBase64(content) ?? ""
}

func precompileIncludeData(_ path: String) -> Data {
    var content: String = ""
    switch (path) {
        // <precompile-content-data>
        default:
            fatalError("Error: include file not found: \(path)")
    }

    return Data.fromBase64(content) ?? Data()
}