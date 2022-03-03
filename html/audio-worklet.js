function memcpy(dst, dst_ofs, src, src_ofs, length) {
    if (length > 0) {
        dst.set(src.subarray(src_ofs, src_ofs + length), dst_ofs);
    }
}

class WASMStreamer extends AudioWorkletProcessor {
    constructor() {
        super();

        this.port.onmessage = (event) => this.init(event.data);
    }

    init(data) {
        this.buffer = data;
    }

    process(_inputs, outputs, params) {
        if (!this.buffer) {
            return true;
        }

        let copied = 0;
        let buflen = this.buffer.buf.length;
        let ipos = this.buffer.ptrs[0];
        let opos = this.buffer.ptrs[1];

        let buffered = (ipos - opos + buflen) % buflen;

        outputs[0].forEach(channel => {
            let to_copy = Math.min(buffered, channel.length);
            copied = Math.max(copied, to_copy);

            let si = opos;
            let ei = (opos + to_copy) % buflen;

            if (ei < si) {
                memcpy(channel, 0,
                       this.buffer.buf, si,
                       buflen - si);
                memcpy(channel, buflen - si,
                       this.buffer.buf, 0,
                       ei);
            } else {
                memcpy(channel, 0,
                       this.buffer.buf, si,
                       ei - si);
            }
        })

        this.buffer.ptrs[1] = (opos + copied) % buflen;

        return true;
    }
}

registerProcessor('wasm-streamer', WASMStreamer);
