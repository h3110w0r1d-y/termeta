import { t } from "../../../i18n";
import { desktop, errorMessage } from "../../../services/desktop/client";
import type { TerminalItem } from "../../workbench/store";
import type { Entry, Stream } from "./entry";

export function createStream(): Stream {
  return {
    seq: 0,
    received: BigInt(0),
    consumed: BigInt(0),
    acked: BigInt(0),
    ack: Promise.resolve(),
    pending: 0,
    inputSeq: 0,
    inputBytes: 0,
    input: Promise.resolve(),
    ended: false,
    endReceived: false,
    eventSeq: 0,
  };
}
export async function acknowledge(
  entry: Entry,
  terminal: TerminalItem,
  stream: Stream,
) {
  if (!stream.sessionId || entry.disposed || stream.consumed <= stream.acked)
    return;
  const offset = stream.consumed;
  stream.acked = offset;
  stream.ack = stream.ack
    .then(() => desktop.ack(stream.sessionId!, offset.toString()))
    .catch((error) => {
      if (entry.stream === stream && !entry.disposed && !stream.ended) {
        terminal.error = errorMessage(error);
      }
    });
  await stream.ack;
}
export function receive(
  entry: Entry,
  terminal: TerminalItem,
  stream: Stream,
  raw: ArrayBuffer,
) {
  if (entry.stream !== stream || entry.disposed) return;
  try {
    if (!(raw instanceof ArrayBuffer) || raw.byteLength < 25)
      throw new Error(t("terminal.errors.outputRead"));
    const view = new DataView(raw),
      length = view.getUint32(16, true),
      seq = view.getUint32(4, true),
      offset = view.getBigUint64(8, true);
    if (
      view.getUint8(0) !== 1 ||
      view.getUint8(1) !== 0 ||
      view.getUint16(2, true) !== 0 ||
      view.getUint32(20, true) !== 0 ||
      length < 1 ||
      length > 32768 ||
      raw.byteLength !== 24 + length ||
      seq !== stream.seq + 1 ||
      offset !== stream.received + BigInt(length) ||
      stream.pending + length > 512 * 1024
    )
      throw new Error(t("terminal.errors.outputFailure"));
    stream.seq = seq;
    stream.received = offset;
    stream.pending += length;
    entry.term.write(new Uint8Array(raw, 24, length), () => {
      stream.pending -= length;
      stream.consumed = offset;
      if (stream.consumed - stream.acked >= BigInt(65536)) {
        if (stream.ackTimer) clearTimeout(stream.ackTimer);
        stream.ackTimer = undefined;
        void acknowledge(entry, terminal, stream);
        return;
      }
      if (stream.ackTimer) return;
      stream.ackTimer = setTimeout(() => {
        stream.ackTimer = undefined;
        void acknowledge(entry, terminal, stream);
      }, 16);
    });
  } catch (error) {
    terminal.error = errorMessage(error);
    terminal.state = "failed";
    stream.ended = true;
    if (stream.sessionId)
      void desktop.close(stream.sessionId, "protocolMismatch").catch(() => {});
  }
}
export function drain(entry: Entry, stream: Stream): Promise<void> {
  return new Promise((resolve, reject) => {
    const deadline = Date.now() + 3000;
    const check = () => {
      if (entry.disposed) return resolve();
      if (
        stream.pending === 0 &&
        (stream.finalOffset === undefined ||
          stream.consumed >= stream.finalOffset)
      ) {
        const timer = setTimeout(
          () => reject(new Error(t("terminal.errors.outputPending"))),
          Math.max(1, deadline - Date.now()),
        );
        entry.term.write(new Uint8Array([0x18]), () => {
          clearTimeout(timer);
          resolve();
        });
        return;
      }
      if (Date.now() >= deadline) {
        reject(new Error(t("terminal.errors.outputPending")));
        return;
      }
      setTimeout(check, 20);
    };
    check();
  });
}
export function input(entry: Entry, terminal: TerminalItem, bytes: Uint8Array) {
  const stream = entry.stream;
  if (
    !stream ||
    entry.closing ||
    stream.ended ||
    terminal.state !== "connected" ||
    !stream.sessionId
  )
    return;
  if (stream.inputBytes + bytes.length > 65536) {
    terminal.error = t("terminal.paste.queueFull");
    return;
  }
  stream.inputBytes += bytes.length;
  for (let at = 0; at < bytes.length; at += 16384) {
    const chunk = bytes.slice(at, at + 16384);
    stream.input = stream.input.then(async () => {
      try {
        if (
          entry.stream !== stream ||
          entry.closing ||
          stream.ended ||
          terminal.state !== "connected"
        )
          return;
        if (stream.inputSeq >= 0xffffffff)
          throw new Error(t("terminal.errors.inputUnavailable"));
        await desktop.write(
          stream.sessionId!,
          ++stream.inputSeq,
          Array.from(chunk),
        );
      } catch (error) {
        stream.ended = true;
        // 在途 write 可能晚于重连返回；旧连接失败不能覆盖新连接的提示。
        if (entry.stream !== stream || entry.disposed) return;
        terminal.error = t("terminal.errors.unsentInput", {
          error: errorMessage(error),
        });
        if (stream.sessionId)
          void desktop.close(stream.sessionId, "inputFailed").catch(() => {});
      } finally {
        stream.inputBytes -= chunk.length;
      }
    });
  }
}
