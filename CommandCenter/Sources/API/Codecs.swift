import Foundation

extension JSONDecoder {
    static var convergio: JSONDecoder {
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        decoder.dateDecodingStrategy = .custom { decoder in
            let container = try decoder.singleValueContainer()
            let value = try container.decode(String.self)
            if let date = makeFormatter(options: [.withInternetDateTime, .withFractionalSeconds]).date(from: value) {
                return date
            }
            if let date = makeFormatter(options: [.withInternetDateTime]).date(from: value) {
                return date
            }
            throw DecodingError.dataCorruptedError(
                in: container,
                debugDescription: "Unsupported timestamp: \(value)"
            )
        }
        return decoder
    }
}

extension JSONEncoder {
    static var convergio: JSONEncoder {
        let encoder = JSONEncoder()
        encoder.keyEncodingStrategy = .convertToSnakeCase
        encoder.dateEncodingStrategy = .iso8601
        return encoder
    }
}

private func makeFormatter(options: ISO8601DateFormatter.Options) -> ISO8601DateFormatter {
    let formatter = ISO8601DateFormatter()
    formatter.formatOptions = options
    return formatter
}
