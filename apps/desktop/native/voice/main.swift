// Armada Voice: hears the owner for Talk, and says what it heard on stdout as one JSON object a line.
// `src/main/voice.ts` runs it and reads the lines. Stdin closing, or SIGTERM, is the release.
//
//   listen                      Apple's on-device recogniser: `partial` lines while he talks, then one `final`
//   whisper <library> <model>   records until the release, then one `final` from whisper.cpp
//   probe                       what macOS holds for it, asking nothing: both permissions and whether on-device recognition is here
//   voices                      one `voice` line for each voice installed, Enhanced and Premium ones too, then `end`
//   speak [<voice id>]          reads a line of text at a time from stdin and says each in turn, with that voice or the Mac's own
//
// `listen` and `whisper` say `ready` once they hear, and end on `final` or `error`.
// `speak` says `speaking` and `spoken` with the line's index, counting from 0, as each starts and ends, and ends on the last
// `spoken` once stdin has closed. A line `{"stop":true}`, or SIGTERM, stops at once and says `stopped`. Speaking asks macOS for nothing.

import AVFoundation
import Foundation
import Speech

func say(_ line: [String: Any]) {
  guard let data = try? JSONSerialization.data(withJSONObject: line), let text = String(data: data, encoding: .utf8) else { return }
  print(text)
}

func fail(_ code: String, _ message: String = "") -> Never {
  say(["kind": "error", "code": code, "message": message])
  exit(1)
}

if voice_run_disclaimed(CommandLine.unsafeArgv) != -1 { fail("respawn_failed") }
setvbuf(stdout, nil, _IOLBF, 0)

func named(_ status: AVAuthorizationStatus) -> String {
  switch status {
  case .authorized: return "granted"
  case .denied: return "denied"
  case .restricted: return "restricted"
  case .notDetermined: return "unasked"
  @unknown default: return "unknown"
  }
}

func named(_ status: SFSpeechRecognizerAuthorizationStatus) -> String {
  switch status {
  case .authorized: return "granted"
  case .denied: return "denied"
  case .restricted: return "restricted"
  case .notDetermined: return "unasked"
  @unknown default: return "unknown"
  }
}

/// Asks once for the microphone, and for speech recognition where `speech`; the answer is kept by macOS after that.
func permitted(speech: Bool, then: @escaping () -> Void) {
  AVCaptureDevice.requestAccess(for: .audio) { granted in
    guard granted else { fail("microphone_denied") }
    guard speech else { return DispatchQueue.main.async(execute: then) }
    SFSpeechRecognizer.requestAuthorization { status in
      guard status == .authorized else { fail("speech_denied", named(status)) }
      DispatchQueue.main.async(execute: then)
    }
  }
}

/// Calls `stop` once, when stdin closes or SIGTERM arrives, whichever is first.
var released = false
var releaseSources: [DispatchSourceSignal] = []
func onRelease(_ stop: @escaping () -> Void) {
  let once = {
    guard !released else { return }
    released = true
    stop()
  }
  for sig in [SIGTERM, SIGINT] {
    signal(sig, SIG_IGN)
    let source = DispatchSource.makeSignalSource(signal: sig, queue: .main)
    source.setEventHandler(handler: once)
    source.resume()
    releaseSources.append(source)
  }
  Thread.detachNewThread {
    while readLine(strippingNewline: true) != nil {}
    DispatchQueue.main.async(execute: once)
  }
}

let engine = AVAudioEngine()

func listen() {
  guard let recognizer = SFSpeechRecognizer() else { fail("no_recognizer", Locale.current.identifier) }
  guard recognizer.supportsOnDeviceRecognition else { fail("on_device_unavailable", recognizer.locale.identifier) }
  let request = SFSpeechAudioBufferRecognitionRequest()
  request.shouldReportPartialResults = true
  request.requiresOnDeviceRecognition = true
  request.addsPunctuation = true
  let input = engine.inputNode
  input.installTap(onBus: 0, bufferSize: 1024, format: input.outputFormat(forBus: 0)) { buffer, _ in request.append(buffer) }
  engine.prepare()
  do { try engine.start() } catch { fail("microphone_unavailable", error.localizedDescription) }
  var heard = ""
  let task = recognizer.recognitionTask(with: request) { result, error in
    if let result {
      heard = result.bestTranscription.formattedString
      say(["kind": result.isFinal ? "final" : "partial", "text": heard])
      if result.isFinal { exit(0) }
    } else if let error {
      // Released before a word: the recogniser calls that an error, and it is not one.
      if released { say(["kind": "final", "text": heard]); exit(0) }
      fail("recognition_failed", error.localizedDescription)
    }
  }
  say(["kind": "ready"])
  onRelease {
    engine.stop()
    input.removeTap(onBus: 0)
    request.endAudio()
    // The final result normally follows in well under a second; past this, what was heard stands.
    DispatchQueue.main.asyncAfter(deadline: .now() + 4) {
      task.cancel()
      say(["kind": "final", "text": heard])
      exit(0)
    }
  }
}

/// At most five minutes are kept, so a held key left down cannot grow the helper without bound.
let WHISPER_RATE = 16_000.0
let WHISPER_MOST = Int(WHISPER_RATE) * 300

func whisper(library: String, model: String) {
  let input = engine.inputNode
  let from = input.outputFormat(forBus: 0)
  guard let to = AVAudioFormat(commonFormat: .pcmFormatFloat32, sampleRate: WHISPER_RATE, channels: 1, interleaved: false),
        let converter = AVAudioConverter(from: from, to: to) else { fail("microphone_unavailable", "no 16 kHz converter") }
  var samples: [Float] = []
  let lock = NSLock()
  input.installTap(onBus: 0, bufferSize: 4096, format: from) { buffer, _ in
    let room = AVAudioFrameCount(Double(buffer.frameLength) * WHISPER_RATE / from.sampleRate) + 1
    guard let out = AVAudioPCMBuffer(pcmFormat: to, frameCapacity: room) else { return }
    var given = false
    converter.convert(to: out, error: nil) { _, status in
      if given { status.pointee = .noDataNow; return nil }
      given = true
      status.pointee = .haveData
      return buffer
    }
    guard let channel = out.floatChannelData?[0] else { return }
    lock.lock()
    if samples.count < WHISPER_MOST { samples.append(contentsOf: UnsafeBufferPointer(start: channel, count: Int(out.frameLength))) }
    lock.unlock()
  }
  engine.prepare()
  do { try engine.start() } catch { fail("microphone_unavailable", error.localizedDescription) }
  say(["kind": "ready"])
  onRelease {
    engine.stop()
    input.removeTap(onBus: 0)
    say(["kind": "transcribing"])
    DispatchQueue.global(qos: .userInitiated).async {
      lock.lock()
      let heard = samples
      lock.unlock()
      // Under a quarter second is a tap, not speech, and whisper invents words for silence.
      guard heard.count > Int(WHISPER_RATE / 4) else { say(["kind": "final", "text": ""]); exit(0) }
      var why: UnsafePointer<CChar>? = nil
      guard let text = heard.withUnsafeBufferPointer({ voice_whisper(library, model, $0.baseAddress, $0.count, &why) }) else {
        fail(why.map { String(cString: $0) } ?? "whisper_failed")
      }
      say(["kind": "final", "text": String(cString: text).trimmingCharacters(in: .whitespacesAndNewlines)])
      free(text)
      exit(0)
    }
  }
}

func probe() {
  say([
    "kind": "probe",
    "pid": Int(getpid()),
    "microphone": named(AVCaptureDevice.authorizationStatus(for: .audio)),
    "speech": named(SFSpeechRecognizer.authorizationStatus()),
    "onDevice": SFSpeechRecognizer()?.supportsOnDeviceRecognition ?? false,
  ])
  exit(0)
}

func voices() {
  let quality: [AVSpeechSynthesisVoiceQuality: String] = [.default: "default", .enhanced: "enhanced", .premium: "premium"]
  let installed = AVSpeechSynthesisVoice.speechVoices().sorted { ($0.language, $0.name) < ($1.language, $1.name) }
  for voice in installed {
    say(["kind": "voice", "id": voice.identifier, "name": voice.name, "language": voice.language, "quality": quality[voice.quality] ?? "default"])
  }
  say(["kind": "end"])
  exit(0)
}

/// Speaks what stdin gives, a line at a time. The synthesizer queues the lines and says them in order.
final class Speaker: NSObject, AVSpeechSynthesizerDelegate, @unchecked Sendable {
  let synthesizer = AVSpeechSynthesizer()
  let voice: AVSpeechSynthesisVoice?
  var indexes: [ObjectIdentifier: Int] = [:]
  var next = 0
  var open = true

  init(voice: AVSpeechSynthesisVoice?) {
    self.voice = voice
    super.init()
    synthesizer.delegate = self
  }

  /// A line the owner's text could never be: a JSON object that says stop.
  static func stops(_ line: String) -> Bool {
    guard line.hasPrefix("{"), let data = line.data(using: .utf8),
          let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return false }
    return object["stop"] as? Bool == true
  }

  func line(_ text: String) {
    if Speaker.stops(text) { return stop() }
    guard !text.trimmingCharacters(in: .whitespaces).isEmpty else { return }
    let utterance = AVSpeechUtterance(string: text)
    utterance.voice = voice
    indexes[ObjectIdentifier(utterance)] = next
    next += 1
    synthesizer.speak(utterance)
  }

  func stop() {
    synthesizer.stopSpeaking(at: .immediate)
    say(["kind": "stopped"])
    exit(0)
  }

  func closed() {
    open = false
    if indexes.isEmpty { exit(0) }
  }

  func speechSynthesizer(_ synthesizer: AVSpeechSynthesizer, didStart utterance: AVSpeechUtterance) {
    if let index = indexes[ObjectIdentifier(utterance)] { say(["kind": "speaking", "index": index]) }
  }

  func speechSynthesizer(_ synthesizer: AVSpeechSynthesizer, didFinish utterance: AVSpeechUtterance) {
    guard let index = indexes.removeValue(forKey: ObjectIdentifier(utterance)) else { return }
    say(["kind": "spoken", "index": index])
    if !open && indexes.isEmpty { exit(0) }
  }
}

var speaker: Speaker?

func speak(voiceId: String?) {
  var chosen: AVSpeechSynthesisVoice?
  if let voiceId {
    guard let found = AVSpeechSynthesisVoice(identifier: voiceId) else { fail("voice_missing", voiceId) }
    chosen = found
  }
  let talker = Speaker(voice: chosen)
  speaker = talker
  for sig in [SIGTERM, SIGINT] {
    signal(sig, SIG_IGN)
    let source = DispatchSource.makeSignalSource(signal: sig, queue: .main)
    source.setEventHandler { talker.stop() }
    source.resume()
    releaseSources.append(source)
  }
  Thread.detachNewThread {
    while let text = readLine(strippingNewline: true) {
      DispatchQueue.main.async { talker.line(text) }
    }
    DispatchQueue.main.async { talker.closed() }
  }
}

let args = Array(CommandLine.arguments.dropFirst())
switch args.first {
case "listen": permitted(speech: true, then: listen)
case "whisper" where args.count == 3: permitted(speech: false) { whisper(library: args[1], model: args[2]) }
case "probe": probe()
case "voices": voices()
case "speak" where args.count <= 2: speak(voiceId: args.count == 2 ? args[1] : nil)
default: fail("usage", "listen | whisper <library> <model> | probe | voices | speak [<voice id>]")
}
dispatchMain()
