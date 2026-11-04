import Foundation
import FoundationModels

// ask [--permissive] [--no-stream] < prompt   -> answer on stdout, JSON timing on stderr
let args = CommandLine.arguments
let permissive = args.contains("--permissive")
let maxTokens: Int? = args.firstIndex(of: "--max").flatMap { Int(args[$0 + 1]) }
let data = FileHandle.standardInput.readDataToEndOfFile()
let prompt = String(decoding: data, as: UTF8.self)

func ms(_ a: ContinuousClock.Instant, _ b: ContinuousClock.Instant) -> Double {
    let d = b - a
    return Double(d.components.seconds) * 1000 + Double(d.components.attoseconds) / 1e15
}

let repeatN: Int = args.firstIndex(of: "--repeat").flatMap { Int(args[$0 + 1]) } ?? 1
let prewarm = args.contains("--prewarm")
@main struct Ask {
    static func main() async {
        for i in 0..<repeatN { await once(i) }
    }
    static func once(_ iter: Int) async {
        let model = permissive
            ? SystemLanguageModel(guardrails: .permissiveContentTransformations)
            : SystemLanguageModel.default
        let session = LanguageModelSession(model: model)
        if prewarm { session.prewarm() }
        let t0 = ContinuousClock.now
        var first: Double? = nil
        var last = ""
        var err = "none"
        do {
            for try await snap in session.streamResponse(to: prompt, options: GenerationOptions(maximumResponseTokens: maxTokens)) {
                if first == nil { first = ms(t0, ContinuousClock.now) }
                last = snap.content
            }
        } catch let e as LanguageModelSession.GenerationError {
            err = "\(e)"
        } catch {
            err = "other: \(error)"
        }
        let total = ms(t0, ContinuousClock.now)
        var toks = -1
        if let t = try? await model.tokenCount(for: prompt) { toks = t }
        var otoks = -1
        if let t = try? await model.tokenCount(for: last) { otoks = t }
        print(last)
        let f = first.map { String(format: "%.0f", $0) } ?? "null"
        let esc = err.replacingOccurrences(of: "\"", with: "'").replacingOccurrences(of: "\n", with: " ")
        FileHandle.standardError.write(Data("{\"iter\":\(iter),\"first_ms\":\(f),\"total_ms\":\(String(format: "%.0f", total)),\"chars\":\(last.count),\"prompt_tokens\":\(toks),\"out_tokens\":\(otoks),\"error\":\"\(esc.prefix(200))\"}\n".utf8))
    }
}
