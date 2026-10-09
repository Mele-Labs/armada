// A Server-Sent Events reader over a fetch body, because `EventSource` cannot carry
// the signing headers. The Gateway sends only `data:` lines, one JSON value each.

export class SseParser {
  private pending = "";
  private data: string[] = [];

  /** Feed a chunk; returns the `data` of every event it completed. */
  feed(chunk: string): string[] {
    this.pending += chunk;
    const lines = this.pending.split(/\r\n|\n|\r/);
    this.pending = lines.pop() ?? "";
    const events: string[] = [];
    for (const line of lines) {
      if (line === "") {
        if (this.data.length > 0) events.push(this.data.join("\n"));
        this.data = [];
      } else if (line.startsWith("data:")) {
        this.data.push(line.slice(5).replace(/^ /, ""));
      }
    }
    return events;
  }
}

export async function* events(body: ReadableStream<Uint8Array>): AsyncGenerator<string> {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  const parser = new SseParser();
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) return;
      yield* parser.feed(decoder.decode(value, { stream: true }));
    }
  } finally {
    reader.releaseLock();
  }
}
