import { decodeSource, encodeSource } from './source-code';

describe('diagram codes', () => {
  it('give back the exact source, comments and all', async () => {
    const source = "@startuml\n' The answer\nDeep -> Thought : 42 ünïcode\n@enduml\n";
    const code = await encodeSource(source);

    expect(code).toMatch(/^[0-9A-Za-z_-]+$/);
    expect(await decodeSource(code)).toBe(source);
  });

  it('read the codes PlantUML writes', async () => {
    expect(await decodeSource('SyfFKj2rKt3CoKnELR1Io4ZDoSa70000')).toBe('Bob -> Alice : hello');
  });

  it('refuse text that is no code', async () => {
    await expect(decodeSource('not a code!')).rejects.toThrow(SyntaxError);
    await expect(decodeSource('0000')).rejects.toThrow(SyntaxError);
  });
});
