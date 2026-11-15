// The Rust engine as WebAssembly (settlers.wasm, built from engine/src by tools/build-engine.mjs),
// the same code the server and the simulations run. Works in the browser and in Node:
//   const engine = await loadEngine(bytes)
//   engine.island('betta', { 'islets.count': 3 })  -> the island (engine/src/island.rs to_json)
//   engine.settingsTable()                          -> [{name, kind, default, min, max, about}]
// Text goes in and out through the module's memory (engine/src/wasm.rs).
export async function loadEngine(bytes) {
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const x = instance.exports;
  const encoder = new TextEncoder(), decoder = new TextDecoder();
  const read = n => decoder.decode(new Uint8Array(x.memory.buffer, x.out(), n));
  const call = (fn, text) => {
    const bytes = encoder.encode(text);
    const p = x.alloc(bytes.length);
    new Uint8Array(x.memory.buffer, p, bytes.length).set(bytes);
    const n = fn(p, bytes.length);
    x.free(p, bytes.length);
    const answer = read(n);
    if (answer.startsWith('error ')) throw new Error(answer.slice(6));
    return answer;
  };
  return {
    // settings: {name: value} for the ones that differ from the defaults
    island(seed, settings = {}) {
      if (String(seed).includes('\n')) throw new Error('a seed is one line');
      const text = Object.entries(settings).map(([k, v]) => `${k}=${v}`).join('\n');
      return JSON.parse(call(x.island, `${seed}\n${text}`));
    },
    settingsTable: () => JSON.parse(read(x.settings_table())),
    sourceHash: () => read(x.source_hash()),

    // Games, kept inside the engine by number (engine/src/wasm.rs). The server uses these; moves
    // are text (engine/src/protocol.rs), e.g. "hut 12", "trade 0 3", "end".
    newGame(seed, players, settings = {}) {
      if (String(seed).includes('\n')) throw new Error('a seed is one line');
      const text = Object.entries(settings).map(([k, v]) => `${k}=${v}`).join('\n');
      return Number(call(x.game_new, `${seed}\n${players}\n${text}`));
    },
    board: id => JSON.parse(check(read(x.game_board(id)))),
    // seat: a number, or null for someone watching
    view: (id, seat) => JSON.parse(check(read(x.game_view(id, seat ?? 255)))),
    // Throws with the reason if the move isn't allowed
    act(id, seat, move) {
      call((p, n) => x.game_act(id, seat, p, n), move);
    },
    // The bot for the seat to act moves; returns its move as text
    bot: id => check(read(x.game_bot(id))),
    // The bot in `seat` answers an open offer made to it: its move as text, or null if nothing to answer
    botAnswer(id, seat) {
      const move = check(read(x.game_bot_answer(id, seat)));
      return move === 'none' ? null : move;
    },
    free: id => x.game_free(id),
  };
}

function check(answer) {
  if (answer.startsWith('error ')) throw new Error(answer.slice(6));
  return answer;
}
