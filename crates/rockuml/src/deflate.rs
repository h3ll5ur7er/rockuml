//! zlib 1.3.1's deflate at level 9 (raw, one call with `Z_FINISH`), as Java's `Deflater(9, true)` runs it.
//!
//! PlantUML embeds deflated sources in URLs and image metadata, so the compressed bytes have to match
//! zlib's exactly; other deflate implementations choose different matches and block boundaries. This is a
//! function-by-function port of `deflate.c` and `trees.c`, restricted to the code paths level 9 uses.

use std::sync::LazyLock;

const MAX_BITS: usize = 15;
const MAX_BL_BITS: usize = 7;
const LITERALS: usize = 256;
const LENGTH_CODES: usize = 29;
const L_CODES: usize = LITERALS + 1 + LENGTH_CODES;
const D_CODES: usize = 30;
const BL_CODES: usize = 19;
const HEAP_SIZE: usize = 2 * L_CODES + 1;
const END_BLOCK: usize = 256;
const REP_3_6: usize = 16;
const REPZ_3_10: usize = 17;
const REPZ_11_138: usize = 18;

const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
const MIN_LOOKAHEAD: usize = MAX_MATCH + MIN_MATCH + 1;
const WINDOW_SIZE: usize = 1 << 15;
const WINDOW_MASK: usize = WINDOW_SIZE - 1;
const MAX_DIST: usize = WINDOW_SIZE - MIN_LOOKAHEAD;
const HASH_BITS: usize = 15;
const HASH_SIZE: usize = 1 << HASH_BITS;
const HASH_MASK: usize = HASH_SIZE - 1;
const HASH_SHIFT: usize = (HASH_BITS + MIN_MATCH - 1) / MIN_MATCH;
const LIT_BUFSIZE: usize = 1 << 14;
const SYM_END: usize = (LIT_BUFSIZE - 1) * 3;
const WIN_INIT: usize = MAX_MATCH;
const TOO_FAR: usize = 4096;
const NIL: usize = 0;

const GOOD_MATCH: usize = 32;
const MAX_LAZY_MATCH: usize = 258;
const NICE_MATCH: usize = 258;
const MAX_CHAIN: usize = 4096;

const STORED_BLOCK: u16 = 0;
const STATIC_TREES: u16 = 1;
const DYN_TREES: u16 = 2;

const EXTRA_LENGTH_BITS: [u16; LENGTH_CODES] =
    [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const EXTRA_DISTANCE_BITS: [u16; D_CODES] =
    [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];
const EXTRA_BIT_LENGTH_BITS: [u16; BL_CODES] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 3, 7];
const BIT_LENGTH_ORDER: [usize; BL_CODES] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

/// zlib's `ct_data`: two unions, frequency/code and parent/length, reused across the tree-building phases.
#[derive(Clone, Copy, Default)]
struct TreeNode {
    freq_or_code: u16,
    dad_or_len: u16,
}

struct StaticTables {
    literal_tree: [TreeNode; L_CODES + 2],
    distance_tree: [TreeNode; D_CODES],
    distance_code: [u8; 512],
    length_code: [u8; MAX_MATCH - MIN_MATCH + 1],
    base_length: [u16; LENGTH_CODES],
    base_distance: [u16; D_CODES],
}

static TABLES: LazyLock<StaticTables> = LazyLock::new(|| {
    let mut length_code = [0u8; MAX_MATCH - MIN_MATCH + 1];
    let mut base_length = [0u16; LENGTH_CODES];
    let mut length = 0;
    for code in 0..LENGTH_CODES - 1 {
        base_length[code] = length as u16;
        for _ in 0..1 << EXTRA_LENGTH_BITS[code] {
            length_code[length] = code as u8;
            length += 1;
        }
    }
    // Length 258 can be coded two ways; zlib prefers code 285.
    length_code[length - 1] = (LENGTH_CODES - 1) as u8;

    let mut distance_code = [0u8; 512];
    let mut base_distance = [0u16; D_CODES];
    let mut distance = 0;
    for code in 0..16 {
        base_distance[code] = distance as u16;
        for _ in 0..1 << EXTRA_DISTANCE_BITS[code] {
            distance_code[distance] = code as u8;
            distance += 1;
        }
    }
    distance >>= 7;
    for code in 16..D_CODES {
        base_distance[code] = (distance << 7) as u16;
        for _ in 0..1 << (EXTRA_DISTANCE_BITS[code] - 7) {
            distance_code[256 + distance] = code as u8;
            distance += 1;
        }
    }

    let mut literal_tree = [TreeNode::default(); L_CODES + 2];
    let mut bit_length_count = [0u16; MAX_BITS + 1];
    for (index, node) in literal_tree.iter_mut().enumerate() {
        node.dad_or_len = match index {
            0..=143 | 280..=287 => 8,
            144..=255 => 9,
            _ => 7,
        };
        bit_length_count[usize::from(node.dad_or_len)] += 1;
    }
    generate_codes(&mut literal_tree, L_CODES + 1, &bit_length_count);

    let mut distance_tree = [TreeNode::default(); D_CODES];
    for (index, node) in distance_tree.iter_mut().enumerate() {
        node.dad_or_len = 5;
        node.freq_or_code = reverse_bits(index as u16, 5);
    }

    StaticTables {
        literal_tree,
        distance_tree,
        distance_code,
        length_code,
        base_length,
        base_distance,
    }
});

fn distance_code(distance: usize) -> usize {
    let tables = &TABLES;
    if distance < 256 {
        usize::from(tables.distance_code[distance])
    } else {
        usize::from(tables.distance_code[256 + (distance >> 7)])
    }
}

fn reverse_bits(code: u16, length: u16) -> u16 {
    let mut code = code;
    let mut result = 0u16;
    for _ in 0..length {
        result = result << 1 | (code & 1);
        code >>= 1;
    }
    result
}

fn generate_codes(tree: &mut [TreeNode], max_code: usize, bit_length_count: &[u16; MAX_BITS + 1]) {
    let mut next_code = [0u16; MAX_BITS + 1];
    let mut code = 0u16;
    for bits in 1..=MAX_BITS {
        code = (code.wrapping_add(bit_length_count[bits - 1])) << 1;
        next_code[bits] = code;
    }
    for node in tree.iter_mut().take(max_code + 1) {
        let length = node.dad_or_len;
        if length == 0 {
            continue;
        }
        node.freq_or_code = reverse_bits(next_code[usize::from(length)], length);
        next_code[usize::from(length)] += 1;
    }
}

#[derive(Clone, Copy)]
enum TreeKind {
    Literal,
    Distance,
    BitLength,
}

/// Compresses `input` as Java's `new Deflater(9, true)` followed by `finish()` and one `deflate` call.
pub fn deflate(input: &[u8]) -> Vec<u8> {
    let mut state = State::new(input);
    state.deflate_slow();
    state.output
}

/// zlib keeps `opt_len` and `static_len` in unsigned longs that may dip below zero while a tree is
/// built; wrapping arithmetic reproduces that.
struct State<'a> {
    input: &'a [u8],
    input_position: usize,
    output: Vec<u8>,

    window: Vec<u8>,
    prev: Vec<u16>,
    head: Vec<u16>,
    insert_hash: usize,
    high_water: usize,
    strstart: usize,
    block_start: isize,
    lookahead: usize,
    insert: usize,
    match_start: usize,
    match_length: usize,
    prev_length: usize,
    prev_match: usize,
    match_available: bool,

    literal_tree: Vec<TreeNode>,
    distance_tree: Vec<TreeNode>,
    bit_length_tree: Vec<TreeNode>,
    literal_max_code: usize,
    distance_max_code: usize,
    heap: Vec<usize>,
    heap_len: usize,
    heap_max: usize,
    depth: Vec<u8>,
    bit_length_count: [u16; MAX_BITS + 1],
    symbols: Vec<u8>,
    symbols_next: usize,
    opt_len: u64,
    static_len: u64,
    bit_buffer: u16,
    bits_valid: u16,
}

impl<'a> State<'a> {
    fn new(input: &'a [u8]) -> Self {
        let mut state = Self {
            input,
            input_position: 0,
            output: Vec::new(),
            window: vec![0; 2 * WINDOW_SIZE],
            prev: vec![0; WINDOW_SIZE],
            head: vec![0; HASH_SIZE],
            insert_hash: 0,
            high_water: 0,
            strstart: 0,
            block_start: 0,
            lookahead: 0,
            insert: 0,
            match_start: 0,
            match_length: MIN_MATCH - 1,
            prev_length: MIN_MATCH - 1,
            prev_match: 0,
            match_available: false,
            literal_tree: vec![TreeNode::default(); HEAP_SIZE],
            distance_tree: vec![TreeNode::default(); 2 * D_CODES + 1],
            bit_length_tree: vec![TreeNode::default(); 2 * BL_CODES + 1],
            literal_max_code: 0,
            distance_max_code: 0,
            heap: vec![0; 2 * L_CODES + 1],
            heap_len: 0,
            heap_max: 0,
            depth: vec![0; 2 * L_CODES + 1],
            bit_length_count: [0; MAX_BITS + 1],
            symbols: vec![0; SYM_END + 3],
            symbols_next: 0,
            opt_len: 0,
            static_len: 0,
            bit_buffer: 0,
            bits_valid: 0,
        };
        state.init_block();
        state
    }

    // ----- deflate.c -----

    fn update_hash(&mut self, byte: u8) {
        self.insert_hash = ((self.insert_hash << HASH_SHIFT) ^ usize::from(byte)) & HASH_MASK;
    }

    /// Inserts the string at `position` into the hash chains and returns the previous chain head.
    fn insert_string(&mut self, position: usize) -> usize {
        self.update_hash(self.window[position + MIN_MATCH - 1]);
        let head = self.head[self.insert_hash];
        self.prev[position & WINDOW_MASK] = head;
        self.head[self.insert_hash] = position as u16;
        usize::from(head)
    }

    fn slide_hash(&mut self) {
        let slide = |position: &mut u16| {
            let value = usize::from(*position);
            *position = if value >= WINDOW_SIZE { (value - WINDOW_SIZE) as u16 } else { NIL as u16 };
        };
        self.head.iter_mut().for_each(slide);
        self.prev.iter_mut().for_each(slide);
    }

    fn read_input(&mut self, destination: usize, size: usize) -> usize {
        let length = size.min(self.input.len() - self.input_position);
        self.window[destination..destination + length]
            .copy_from_slice(&self.input[self.input_position..self.input_position + length]);
        self.input_position += length;
        length
    }

    fn has_input(&self) -> bool {
        self.input_position < self.input.len()
    }

    fn fill_window(&mut self) {
        loop {
            let mut more = 2 * WINDOW_SIZE - self.lookahead - self.strstart;
            if self.strstart >= WINDOW_SIZE + MAX_DIST {
                self.window.copy_within(WINDOW_SIZE..2 * WINDOW_SIZE - more, 0);
                self.match_start = self.match_start.wrapping_sub(WINDOW_SIZE);
                self.strstart -= WINDOW_SIZE;
                self.block_start -= WINDOW_SIZE as isize;
                if self.insert > self.strstart {
                    self.insert = self.strstart;
                }
                self.slide_hash();
                more += WINDOW_SIZE;
            }
            if !self.has_input() {
                break;
            }
            let read = self.read_input(self.strstart + self.lookahead, more);
            self.lookahead += read;

            if self.lookahead + self.insert >= MIN_MATCH {
                let mut position = self.strstart - self.insert;
                self.insert_hash = usize::from(self.window[position]);
                self.update_hash(self.window[position + 1]);
                while self.insert > 0 {
                    self.update_hash(self.window[position + MIN_MATCH - 1]);
                    self.prev[position & WINDOW_MASK] = self.head[self.insert_hash];
                    self.head[self.insert_hash] = position as u16;
                    position += 1;
                    self.insert -= 1;
                    if self.lookahead + self.insert < MIN_MATCH {
                        break;
                    }
                }
            }
            if self.lookahead >= MIN_LOOKAHEAD || !self.has_input() {
                break;
            }
        }

        let window_size = 2 * WINDOW_SIZE;
        if self.high_water < window_size {
            let current = self.strstart + self.lookahead;
            if self.high_water < current {
                let init = (window_size - current).min(WIN_INIT);
                self.window[current..current + init].fill(0);
                self.high_water = current + init;
            } else if self.high_water < current + WIN_INIT {
                let init = (current + WIN_INIT - self.high_water).min(window_size - self.high_water);
                self.window[self.high_water..self.high_water + init].fill(0);
                self.high_water += init;
            }
        }
    }

    fn longest_match(&mut self, mut current_match: usize) -> usize {
        let mut chain_length = MAX_CHAIN;
        let scan_start = self.strstart;
        let mut best_length = self.prev_length;
        let mut nice_match = NICE_MATCH;
        let limit = if self.strstart > MAX_DIST { self.strstart - MAX_DIST } else { NIL };
        let string_end = self.strstart + MAX_MATCH;
        let mut scan_end_1 = self.window[scan_start + best_length - 1];
        let mut scan_end = self.window[scan_start + best_length];

        if self.prev_length >= GOOD_MATCH {
            chain_length >>= 2;
        }
        if nice_match > self.lookahead {
            nice_match = self.lookahead;
        }

        loop {
            let candidate = current_match;
            let window = &self.window;
            if window[candidate + best_length] == scan_end
                && window[candidate + best_length - 1] == scan_end_1
                && window[candidate] == window[scan_start]
                && window[candidate + 1] == window[scan_start + 1]
            {
                // Bytes 2 are equal whenever the hashes are; the comparison resumes at 3.
                let mut scan = scan_start + 2;
                let mut matched = candidate + 2;
                loop {
                    scan += 1;
                    matched += 1;
                    if window[scan] != window[matched] || scan >= string_end {
                        break;
                    }
                }
                let length = MAX_MATCH - (string_end - scan);
                if length > best_length {
                    self.match_start = candidate;
                    best_length = length;
                    if length >= nice_match {
                        break;
                    }
                    scan_end_1 = window[scan_start + best_length - 1];
                    scan_end = window[scan_start + best_length];
                }
            }
            current_match = usize::from(self.prev[current_match & WINDOW_MASK]);
            chain_length -= 1;
            if current_match <= limit || chain_length == 0 {
                break;
            }
        }
        best_length.min(self.lookahead)
    }

    fn flush_block(&mut self, last: bool) {
        let stored = usize::try_from(self.block_start)
            .ok()
            .map(|start| (start, self.strstart - start));
        self.tr_flush_block(stored, (self.strstart as isize - self.block_start) as u64, last);
        self.block_start = self.strstart as isize;
    }

    fn deflate_slow(&mut self) {
        let mut flush_needed;
        loop {
            if self.lookahead < MIN_LOOKAHEAD {
                self.fill_window();
                if self.lookahead == 0 {
                    break;
                }
            }

            let mut hash_head = NIL;
            if self.lookahead >= MIN_MATCH {
                hash_head = self.insert_string(self.strstart);
            }

            self.prev_length = self.match_length;
            self.prev_match = self.match_start;
            self.match_length = MIN_MATCH - 1;

            if hash_head != NIL && self.prev_length < MAX_LAZY_MATCH && self.strstart - hash_head <= MAX_DIST {
                self.match_length = self.longest_match(hash_head);
                if self.match_length <= 5
                    && self.match_length == MIN_MATCH
                    && self.strstart - self.match_start > TOO_FAR
                {
                    self.match_length = MIN_MATCH - 1;
                }
            }

            if self.prev_length >= MIN_MATCH && self.match_length <= self.prev_length {
                let max_insert = self.strstart + self.lookahead - MIN_MATCH;
                flush_needed = self.tally_distance(self.strstart - 1 - self.prev_match, self.prev_length - MIN_MATCH);
                self.lookahead -= self.prev_length - 1;
                self.prev_length -= 2;
                loop {
                    self.strstart += 1;
                    if self.strstart <= max_insert {
                        self.insert_string(self.strstart);
                    }
                    self.prev_length -= 1;
                    if self.prev_length == 0 {
                        break;
                    }
                }
                self.match_available = false;
                self.match_length = MIN_MATCH - 1;
                self.strstart += 1;
                if flush_needed {
                    self.flush_block(false);
                }
            } else if self.match_available {
                flush_needed = self.tally_literal(self.window[self.strstart - 1]);
                if flush_needed {
                    self.flush_block(false);
                }
                self.strstart += 1;
                self.lookahead -= 1;
            } else {
                self.match_available = true;
                self.strstart += 1;
                self.lookahead -= 1;
            }
        }
        if self.match_available {
            self.tally_literal(self.window[self.strstart - 1]);
            self.match_available = false;
        }
        self.insert = self.strstart.min(MIN_MATCH - 1);
        self.flush_block(true);
    }

    // ----- trees.c -----

    fn tree(&mut self, kind: TreeKind) -> &mut Vec<TreeNode> {
        match kind {
            TreeKind::Literal => &mut self.literal_tree,
            TreeKind::Distance => &mut self.distance_tree,
            TreeKind::BitLength => &mut self.bit_length_tree,
        }
    }

    fn init_block(&mut self) {
        self.literal_tree.iter_mut().take(L_CODES).for_each(|node| node.freq_or_code = 0);
        self.distance_tree.iter_mut().take(D_CODES).for_each(|node| node.freq_or_code = 0);
        self.bit_length_tree.iter_mut().take(BL_CODES).for_each(|node| node.freq_or_code = 0);
        self.literal_tree[END_BLOCK].freq_or_code = 1;
        self.opt_len = 0;
        self.static_len = 0;
        self.symbols_next = 0;
    }

    fn tally_literal(&mut self, byte: u8) -> bool {
        self.symbols[self.symbols_next..self.symbols_next + 3].copy_from_slice(&[0, 0, byte]);
        self.symbols_next += 3;
        self.literal_tree[usize::from(byte)].freq_or_code += 1;
        self.symbols_next == SYM_END
    }

    fn tally_distance(&mut self, distance: usize, length: usize) -> bool {
        let [low, high] = (distance as u16).to_le_bytes();
        self.symbols[self.symbols_next..self.symbols_next + 3].copy_from_slice(&[low, high, length as u8]);
        self.symbols_next += 3;
        let length_symbol = usize::from(TABLES.length_code[length]) + LITERALS + 1;
        self.literal_tree[length_symbol].freq_or_code += 1;
        self.distance_tree[distance_code(distance - 1)].freq_or_code += 1;
        self.symbols_next == SYM_END
    }

    fn put_byte(&mut self, byte: u8) {
        self.output.push(byte);
    }

    fn put_short(&mut self, value: u16) {
        self.output.extend_from_slice(&value.to_le_bytes());
    }

    /// zlib shifts in `int` and truncates to 16 bits, so bits shifted past the buffer are simply lost.
    fn send_bits(&mut self, value: u16, length: u16) {
        let shifted = (u32::from(value) << self.bits_valid) as u16;
        if self.bits_valid > 16 - length {
            self.bit_buffer |= shifted;
            self.put_short(self.bit_buffer);
            self.bit_buffer = value >> (16 - self.bits_valid);
            self.bits_valid = self.bits_valid + length - 16;
        } else {
            self.bit_buffer |= shifted;
            self.bits_valid += length;
        }
    }

    fn send_code(&mut self, code: usize, kind: TreeKind) {
        let node = self.tree(kind)[code];
        self.send_bits(node.freq_or_code, node.dad_or_len);
    }

    fn send_static_code(&mut self, node: TreeNode) {
        self.send_bits(node.freq_or_code, node.dad_or_len);
    }

    fn bits_windup(&mut self) {
        if self.bits_valid > 8 {
            self.put_short(self.bit_buffer);
        } else if self.bits_valid > 0 {
            self.put_byte(self.bit_buffer as u8);
        }
        self.bit_buffer = 0;
        self.bits_valid = 0;
    }

    fn smaller(tree: &[TreeNode], depth: &[u8], n: usize, m: usize) -> bool {
        tree[n].freq_or_code < tree[m].freq_or_code
            || (tree[n].freq_or_code == tree[m].freq_or_code && depth[n] <= depth[m])
    }

    fn down_heap(&mut self, kind: TreeKind, mut k: usize) {
        let tree = match kind {
            TreeKind::Literal => &self.literal_tree,
            TreeKind::Distance => &self.distance_tree,
            TreeKind::BitLength => &self.bit_length_tree,
        };
        let v = self.heap[k];
        let mut j = k << 1;
        while j <= self.heap_len {
            if j < self.heap_len && Self::smaller(tree, &self.depth, self.heap[j + 1], self.heap[j]) {
                j += 1;
            }
            if Self::smaller(tree, &self.depth, v, self.heap[j]) {
                break;
            }
            self.heap[k] = self.heap[j];
            k = j;
            j <<= 1;
        }
        self.heap[k] = v;
    }

    fn tree_description(kind: TreeKind) -> (Option<&'static [TreeNode]>, &'static [u16], usize, usize, usize) {
        match kind {
            TreeKind::Literal => (Some(&TABLES.literal_tree[..]), &EXTRA_LENGTH_BITS, LITERALS + 1, L_CODES, MAX_BITS),
            TreeKind::Distance => (Some(&TABLES.distance_tree[..]), &EXTRA_DISTANCE_BITS, 0, D_CODES, MAX_BITS),
            TreeKind::BitLength => (None, &EXTRA_BIT_LENGTH_BITS, 0, BL_CODES, MAX_BL_BITS),
        }
    }

    fn generate_bit_lengths(&mut self, kind: TreeKind, max_code: usize) {
        let (static_tree, extra, base, _, max_length) = Self::tree_description(kind);
        self.bit_length_count = [0; MAX_BITS + 1];
        let mut overflow = 0;
        let root = self.heap[self.heap_max];
        self.tree(kind)[root].dad_or_len = 0;

        let mut h = self.heap_max + 1;
        while h < HEAP_SIZE {
            let n = self.heap[h];
            let dad = usize::from(self.tree(kind)[n].dad_or_len);
            let mut bits = usize::from(self.tree(kind)[dad].dad_or_len) + 1;
            if bits > max_length {
                bits = max_length;
                overflow += 1;
            }
            self.tree(kind)[n].dad_or_len = bits as u16;
            h += 1;
            if n > max_code {
                continue;
            }
            self.bit_length_count[bits] += 1;
            let extra_bits = if n >= base { usize::from(extra[n - base]) } else { 0 };
            let frequency = u64::from(self.tree(kind)[n].freq_or_code);
            self.opt_len = self.opt_len.wrapping_add(frequency * (bits + extra_bits) as u64);
            if let Some(static_tree) = static_tree {
                let static_bits = (usize::from(static_tree[n].dad_or_len) + extra_bits) as u64;
                self.static_len = self.static_len.wrapping_add(frequency * static_bits);
            }
        }
        if overflow == 0 {
            return;
        }

        loop {
            let mut bits = max_length - 1;
            while self.bit_length_count[bits] == 0 {
                bits -= 1;
            }
            self.bit_length_count[bits] -= 1;
            self.bit_length_count[bits + 1] += 2;
            self.bit_length_count[max_length] -= 1;
            overflow -= 2;
            if overflow <= 0 {
                break;
            }
        }

        let mut h = HEAP_SIZE;
        for bits in (1..=max_length).rev() {
            let mut n = self.bit_length_count[bits];
            while n != 0 {
                h -= 1;
                let m = self.heap[h];
                if m > max_code {
                    continue;
                }
                let node = &mut self.tree(kind)[m];
                if usize::from(node.dad_or_len) != bits {
                    let frequency = u64::from(node.freq_or_code);
                    let old_bits = u64::from(node.dad_or_len);
                    node.dad_or_len = bits as u16;
                    self.opt_len = self.opt_len.wrapping_add((bits as u64).wrapping_sub(old_bits).wrapping_mul(frequency));
                }
                n -= 1;
            }
        }
    }

    /// Builds one Huffman tree and returns its largest code with a non-zero frequency.
    fn build_tree(&mut self, kind: TreeKind) -> usize {
        let (static_tree, _, _, elements, _) = Self::tree_description(kind);
        let mut max_code: isize = -1;
        self.heap_len = 0;
        self.heap_max = HEAP_SIZE;

        for n in 0..elements {
            if self.tree(kind)[n].freq_or_code != 0 {
                self.heap_len += 1;
                self.heap[self.heap_len] = n;
                max_code = n as isize;
                self.depth[n] = 0;
            } else {
                self.tree(kind)[n].dad_or_len = 0;
            }
        }

        while self.heap_len < 2 {
            let node = if max_code < 2 {
                max_code += 1;
                max_code as usize
            } else {
                0
            };
            self.heap_len += 1;
            self.heap[self.heap_len] = node;
            self.tree(kind)[node].freq_or_code = 1;
            self.depth[node] = 0;
            self.opt_len = self.opt_len.wrapping_sub(1);
            if let Some(static_tree) = static_tree {
                self.static_len = self.static_len.wrapping_sub(u64::from(static_tree[node].dad_or_len));
            }
        }
        let max_code = max_code as usize;

        for n in (1..=self.heap_len / 2).rev() {
            self.down_heap(kind, n);
        }

        let mut node = elements;
        loop {
            let n = self.heap[1];
            self.heap[1] = self.heap[self.heap_len];
            self.heap_len -= 1;
            self.down_heap(kind, 1);
            let m = self.heap[1];

            self.heap_max -= 1;
            self.heap[self.heap_max] = n;
            self.heap_max -= 1;
            self.heap[self.heap_max] = m;

            let tree = self.tree(kind);
            tree[node].freq_or_code = tree[n].freq_or_code.wrapping_add(tree[m].freq_or_code);
            tree[n].dad_or_len = node as u16;
            tree[m].dad_or_len = node as u16;
            self.depth[node] = self.depth[n].max(self.depth[m]) + 1;
            self.heap[1] = node;
            node += 1;
            self.down_heap(kind, 1);
            if self.heap_len < 2 {
                break;
            }
        }
        self.heap_max -= 1;
        self.heap[self.heap_max] = self.heap[1];

        self.generate_bit_lengths(kind, max_code);
        let counts = self.bit_length_count;
        generate_codes(self.tree(kind), max_code, &counts);
        max_code
    }

    fn scan_tree(&mut self, kind: TreeKind, max_code: usize) {
        let mut previous_length: isize = -1;
        let mut next_length = self.tree(kind)[0].dad_or_len;
        let mut count = 0;
        let (mut max_count, mut min_count) = if next_length == 0 { (138, 3) } else { (7, 4) };
        self.tree(kind)[max_code + 1].dad_or_len = 0xFFFF;

        for n in 0..=max_code {
            let current_length = next_length;
            next_length = self.tree(kind)[n + 1].dad_or_len;
            count += 1;
            if count < max_count && current_length == next_length {
                continue;
            } else if count < min_count {
                self.bit_length_tree[usize::from(current_length)].freq_or_code += count;
            } else if current_length != 0 {
                if current_length as isize != previous_length {
                    self.bit_length_tree[usize::from(current_length)].freq_or_code += 1;
                }
                self.bit_length_tree[REP_3_6].freq_or_code += 1;
            } else if count <= 10 {
                self.bit_length_tree[REPZ_3_10].freq_or_code += 1;
            } else {
                self.bit_length_tree[REPZ_11_138].freq_or_code += 1;
            }
            count = 0;
            previous_length = current_length as isize;
            (max_count, min_count) = if next_length == 0 {
                (138, 3)
            } else if current_length == next_length {
                (6, 3)
            } else {
                (7, 4)
            };
        }
    }

    fn send_tree(&mut self, kind: TreeKind, max_code: usize) {
        let mut previous_length: isize = -1;
        let mut next_length = self.tree(kind)[0].dad_or_len;
        let mut count: u16 = 0;
        let (mut max_count, mut min_count) = if next_length == 0 { (138, 3) } else { (7, 4) };

        for n in 0..=max_code {
            let current_length = next_length;
            next_length = self.tree(kind)[n + 1].dad_or_len;
            count += 1;
            if count < max_count && current_length == next_length {
                continue;
            } else if count < min_count {
                for _ in 0..count {
                    self.send_code(usize::from(current_length), TreeKind::BitLength);
                }
            } else if current_length != 0 {
                if current_length as isize != previous_length {
                    self.send_code(usize::from(current_length), TreeKind::BitLength);
                    count -= 1;
                }
                self.send_code(REP_3_6, TreeKind::BitLength);
                self.send_bits(count - 3, 2);
            } else if count <= 10 {
                self.send_code(REPZ_3_10, TreeKind::BitLength);
                self.send_bits(count - 3, 3);
            } else {
                self.send_code(REPZ_11_138, TreeKind::BitLength);
                self.send_bits(count - 11, 7);
            }
            count = 0;
            previous_length = current_length as isize;
            (max_count, min_count) = if next_length == 0 {
                (138, 3)
            } else if current_length == next_length {
                (6, 3)
            } else {
                (7, 4)
            };
        }
    }

    /// Returns the index in `BIT_LENGTH_ORDER` of the last bit-length code to send.
    fn build_bit_length_tree(&mut self) -> usize {
        self.scan_tree(TreeKind::Literal, self.literal_max_code);
        self.scan_tree(TreeKind::Distance, self.distance_max_code);
        self.build_tree(TreeKind::BitLength);
        let mut max_index = BL_CODES - 1;
        while max_index >= 3 && self.bit_length_tree[BIT_LENGTH_ORDER[max_index]].dad_or_len == 0 {
            max_index -= 1;
        }
        self.opt_len = self.opt_len.wrapping_add(3 * (max_index as u64 + 1) + 5 + 5 + 4);
        max_index
    }

    fn send_all_trees(&mut self, literal_codes: usize, distance_codes: usize, bit_length_codes: usize) {
        self.send_bits((literal_codes - 257) as u16, 5);
        self.send_bits((distance_codes - 1) as u16, 5);
        self.send_bits((bit_length_codes - 4) as u16, 4);
        for &code in BIT_LENGTH_ORDER.iter().take(bit_length_codes) {
            self.send_bits(self.bit_length_tree[code].dad_or_len, 3);
        }
        self.send_tree(TreeKind::Literal, literal_codes - 1);
        self.send_tree(TreeKind::Distance, distance_codes - 1);
    }

    fn compress_block(&mut self, dynamic: bool) {
        let tables = &*TABLES;
        let mut index = 0;
        while index < self.symbols_next {
            let distance = usize::from(u16::from_le_bytes([self.symbols[index], self.symbols[index + 1]]));
            let length_or_literal = usize::from(self.symbols[index + 2]);
            index += 3;
            let literal_node = |state: &Self, symbol: usize| {
                if dynamic { state.literal_tree[symbol] } else { tables.literal_tree[symbol] }
            };
            if distance == 0 {
                let node = literal_node(self, length_or_literal);
                self.send_static_code(node);
                continue;
            }
            let code = usize::from(tables.length_code[length_or_literal]);
            let node = literal_node(self, code + LITERALS + 1);
            self.send_static_code(node);
            let extra = EXTRA_LENGTH_BITS[code];
            if extra != 0 {
                self.send_bits((length_or_literal - usize::from(tables.base_length[code])) as u16, extra);
            }
            let distance = distance - 1;
            let code = distance_code(distance);
            let node = if dynamic { self.distance_tree[code] } else { tables.distance_tree[code] };
            self.send_static_code(node);
            let extra = EXTRA_DISTANCE_BITS[code];
            if extra != 0 {
                self.send_bits((distance - usize::from(tables.base_distance[code])) as u16, extra);
            }
        }
        let node = if dynamic { self.literal_tree[END_BLOCK] } else { tables.literal_tree[END_BLOCK] };
        self.send_static_code(node);
    }

    fn tr_stored_block(&mut self, start: usize, length: usize, last: bool) {
        self.send_bits((STORED_BLOCK << 1) + u16::from(last), 3);
        self.bits_windup();
        self.put_short(length as u16);
        self.put_short(!(length as u16));
        let window = &self.window[start..start + length];
        self.output.extend_from_slice(window);
    }

    /// `stored` is the block's position in the window, if it is still there to be emitted uncompressed.
    fn tr_flush_block(&mut self, stored: Option<(usize, usize)>, stored_length: u64, last: bool) {
        self.literal_max_code = self.build_tree(TreeKind::Literal);
        self.distance_max_code = self.build_tree(TreeKind::Distance);
        let max_bit_length_index = self.build_bit_length_tree();

        let mut opt_length_bytes = (self.opt_len + 3 + 7) >> 3;
        let static_length_bytes = (self.static_len + 3 + 7) >> 3;
        if static_length_bytes <= opt_length_bytes {
            opt_length_bytes = static_length_bytes;
        }

        match stored {
            Some((start, length)) if stored_length + 4 <= opt_length_bytes => {
                self.tr_stored_block(start, length, last);
            }
            _ if static_length_bytes == opt_length_bytes => {
                self.send_bits((STATIC_TREES << 1) + u16::from(last), 3);
                self.compress_block(false);
            }
            _ => {
                self.send_bits((DYN_TREES << 1) + u16::from(last), 3);
                self.send_all_trees(self.literal_max_code + 1, self.distance_max_code + 1, max_bit_length_index + 1);
                self.compress_block(true);
            }
        }
        self.init_block();
        if last {
            self.bits_windup();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fnv(data: &[u8]) -> u64 {
        data.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3))
    }

    fn text(lines: u32) -> Vec<u8> {
        (0..lines)
            .flat_map(|i| format!("line {i} with some text {}\n", i * i % 97).into_bytes())
            .collect()
    }

    fn random(length: usize, seed: i32) -> Vec<u8> {
        let mut x = seed;
        (0..length)
            .map(|_| {
                x = x.wrapping_mul(1_103_515_245).wrapping_add(12345);
                (x.cast_unsigned() >> 16) as u8
            })
            .collect()
    }

    /// Output length and FNV-1a hash of Java's `Deflater(9, true)` for the same inputs.
    #[test]
    fn matches_java_deflater_byte_for_byte() {
        let mut mixed = random(40_000, 3);
        mixed.extend_from_slice(&text(2000)[..50_000]);
        let cases: [(&str, Vec<u8>, usize, u64); 7] = [
            ("hello", b"Bob -> Alice : hello".to_vec(), 22, 0xe61b_03a1_dc6b_a011),
            ("single", b"x".to_vec(), 3, 0x66d0_851a_6f36_a3da),
            ("run", vec![b'a'; 82], 6, 0x8e03_738e_ea33_3f7c),
            ("text_small", text(30), 170, 0x0868_74f2_ab05_5774),
            ("text_large", text(5000), 19_116, 0xc289_19ea_29a3_2960),
            ("random", random(100_000, 7), 100_035, 0x7151_ab7b_c6fe_978b),
            ("mixed", mixed, 48_392, 0xc900_d002_5fa6_5ab7),
        ];
        for (name, input, length, hash) in cases {
            let output = deflate(&input);
            assert_eq!((output.len(), fnv(&output)), (length, hash), "{name}");
        }
    }

    #[test]
    fn decompresses_back_to_the_input() {
        use std::io::Read;
        let input = text(8000);
        let mut decoded = Vec::new();
        flate2::read::DeflateDecoder::new(&deflate(&input)[..]).read_to_end(&mut decoded).unwrap();
        assert_eq!(decoded, input);
    }
}
