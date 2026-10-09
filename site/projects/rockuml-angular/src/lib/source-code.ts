// Diagram sources as URL-safe text, for links that carry a diagram.
//
// The text is the source deflated and written in PlantUML's 64-letter alphabet, so PlantUML servers decode it
// too. Unlike PlantUML's own encoder this one keeps the source exactly, comments and `@start` lines included.

const ALPHABET = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-_';

export async function encodeSource(source: string): Promise<string> {
  const deflated = await transform(
    new TextEncoder().encode(source),
    new CompressionStream('deflate-raw'),
  );
  let code = '';
  for (let index = 0; index < deflated.length; index += 3) {
    const [b1 = 0, b2 = 0, b3 = 0] = deflated.subarray(index, index + 3);
    code += [b1 >> 2, ((b1 & 0x3) << 4) | (b2 >> 4), ((b2 & 0xf) << 2) | (b3 >> 6), b3 & 0x3f]
      .map((sextet) => ALPHABET[sextet])
      .join('');
  }
  return code;
}

/** The source of a code `encodeSource` made; throws if `code` is no such code. */
export async function decodeSource(code: string): Promise<string> {
  const bytes: number[] = [];
  for (let index = 0; index < code.length; index += 4) {
    const [c1, c2, c3, c4] = Array.from({ length: 4 }, (_, offset) =>
      sextet(code[index + offset] ?? '0'),
    );
    bytes.push((c1 << 2) | (c2 >> 4), ((c2 & 0xf) << 4) | (c3 >> 2), ((c3 & 0x3) << 6) | c4);
  }
  // The last letters carry up to two bytes of padding, which browsers refuse as data after the deflated stream.
  // Only the length without them inflates: shorter cuts the stream off, longer has the padding.
  for (const padding of [2, 1, 0]) {
    try {
      const deflated = Uint8Array.from(bytes.slice(0, bytes.length - padding));
      const inflated = await transform(deflated, new DecompressionStream('deflate-raw'));
      return new TextDecoder().decode(inflated);
    } catch {
      // Not this length.
    }
  }
  throw new SyntaxError('not a diagram code');
}

function sextet(letter: string): number {
  const value = ALPHABET.indexOf(letter);
  if (value < 0) {
    throw new SyntaxError(`${letter} is not part of a diagram code`);
  }
  return value;
}

async function transform(
  data: Uint8Array<ArrayBuffer>,
  stream: CompressionStream | DecompressionStream,
): Promise<Uint8Array> {
  const output = new Response(data).body!.pipeThrough(stream);
  return new Uint8Array(await new Response(output).arrayBuffer());
}
