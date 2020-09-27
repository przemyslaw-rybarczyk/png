use crate::file::ByteReader;
use crate::file::BitReader;
use crate::idat::IdatReader;
use crate::Error;
use crate::Result;

enum HuffmanCodes {
    Value(u16),
    Branch(Box<HuffmanCodes>, Box<HuffmanCodes>),
}

impl HuffmanCodes {
    fn new(lengths: &mut [(u16, u16)]) -> Result<HuffmanCodes> {
        fn rec(lengths: &[(u16, u16)], i: usize, depth: u16) -> Result<(HuffmanCodes, usize)> {
            if i >= lengths.len() {
                return Err(Error::Format("Invalid Huffman codes"));
            }
            let (len, val) = lengths[i]; // invariant: len >= depth
            if len > depth {
                let (codes_l, i1) = rec(lengths, i, depth + 1)?;
                let (codes_r, i2) = rec(lengths, i1, depth + 1)?;
                Ok((HuffmanCodes::Branch(Box::new(codes_l), Box::new(codes_r)), i2))
            } else {
                Ok((HuffmanCodes::Value(val), i + 1))
            }
        }
        lengths.sort();
        let i = lengths.iter().take_while(|(l, _)| *l == 0).count();
        let (codes, i) = rec(lengths, i, 0)?;
        if i < lengths.len() {
            return Err(Error::Format("Invalid Huffman codes"));
        }
        Ok(codes)
    }
}

fn read_huffman<T>(reader: &mut BitReader<T>, mut codes: &HuffmanCodes) -> Result<u16> where T: ByteReader {
    loop {
        match codes {
            HuffmanCodes::Value(val) => return Ok(*val),
            HuffmanCodes::Branch(codes_l, codes_r) => codes = match reader.read_bit()? {
                false => codes_l,
                true => codes_r,
            }
        }
    }
}

enum BlockType {
    Uncompressed(u16),
    Huffman(HuffmanCodes, HuffmanCodes),
}

const CODE_LENGTH_ORDER: [u16; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

fn next_block(idat: &mut BitReader<IdatReader>) -> Result<(bool, BlockType)> {
    println!("");
    let bfinal = idat.read_bit()?;
    let btype = idat.read_bits(2)?;
    println!("Final block: {}", bfinal as u32);
    println!("Block type: {}", btype);
    let btype = match btype {
        0 => {
            let len = idat.read_u16()?;
            println!("Block length: {}", len);
            let nlen = idat.read_u16()?;
            println!("One's complement of block's length: {}", nlen);
            if !len != nlen {
                warn!("One's complement of block length is incorrect");
            }
            BlockType::Uncompressed(len)
        },
        1 => {
            let mut literals_lengths: Box<[(u16, u16)]> = vec![(0, 0); 288].into_boxed_slice();
            for (i, v) in literals_lengths.iter_mut().enumerate() {
                *v = (if i < 144 { 8 } else if i < 256 { 9 } else if i < 280 { 7 } else { 8 }, i as u16);
            }
            let mut distances_lengths = vec![(0, 0); 32].into_boxed_slice();
            for (i, v) in distances_lengths.iter_mut().enumerate() {
                *v = (5, i as u16);
            }
            BlockType::Huffman(HuffmanCodes::new(&mut literals_lengths)?, HuffmanCodes::new(&mut distances_lengths)?)
        },
        2 => {
            let literals_num = (idat.read_bits(5)? as usize) + 257;
            println!("Number of literal/length codes: {}", literals_num);
            let distances_num = (idat.read_bits(5)? as usize) + 1;
            println!("Number of distance codes: {}", distances_num);
            let code_lengths_num = (idat.read_bits(4)? as usize) + 4;
            println!("Number of code length codes: {}", code_lengths_num);
            let mut code_lengths_lengths = vec![(0, 0); code_lengths_num].into_boxed_slice();
            for (i, v) in code_lengths_lengths.iter_mut().enumerate() {
                *v = (idat.read_bits(3)?, CODE_LENGTH_ORDER[i]);
            }
            let code_lengths_codes = HuffmanCodes::new(&mut code_lengths_lengths)?;
            let mut literals_distances_lengths = vec![(0, 0); literals_num + distances_num].into_boxed_slice();
            {
                let mut repeated = None;
                let mut repeats = 0;
                for (i, v) in literals_distances_lengths.iter_mut().enumerate() {
                    if repeats == 0 {
                        let repeat = match read_huffman(idat, &code_lengths_codes)? {
                            16 => (repeated, idat.read_bits(2)? + 3),
                            17 => (Some(0), idat.read_bits(3)? + 3),
                            18 => (Some(0), idat.read_bits(7)? + 11),
                            val => (Some(val), 1),
                        };
                        repeated = repeat.0;
                        repeats = repeat.1;
                    }
                    match repeated {
                        Some(val) => *v = (val, if i < literals_num {i} else {i - literals_num} as u16),
                        None => return Err(Error::Format("Code length alphabet symbol 16 occurs at the beginning")),
                    };
                    repeats -= 1;
                }
                if repeats != 0 {
                    warn!("Code length table specified beyond end");
                }
            }
            // TODO handle case of only one distance code
            let literal_codes = HuffmanCodes::new(&mut literals_distances_lengths[..literals_num])?;
            let distance_codes = HuffmanCodes::new(&mut literals_distances_lengths[literals_num..])?;
            BlockType::Huffman(literal_codes, distance_codes)
        },
        _ => return Err(Error::Format("Invalid block type")),
    };
    Ok((bfinal, btype))
}

const length_code_interpretation: [(usize, u8); 29] = [
    (3, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0), (10, 0),
    (11, 1), (13, 1), (15, 1), (17, 1),
    (19, 2), (23, 2), (27, 2), (31, 2),
    (35, 3), (43, 3), (51, 3), (59, 3),
    (67, 4), (83, 4), (99, 4), (115, 4),
    (131, 5), (163, 5), (195, 5), (227, 5),
    (258, 0),
];

const distance_code_interpretation: [(usize, u8); 30] = [
    (1, 0), (2, 0), (3, 0), (4, 0),
    (5, 1), (7, 1), (9, 2), (13, 2),
    (17, 3), (25, 3), (33, 4), (49, 4),
    (65, 5), (97, 5), (129, 6), (193, 6),
    (257, 7), (385, 7), (513, 8), (769, 8),
    (1025, 9), (1537, 9), (2049, 10), (3073, 10),
    (4097, 11), (6145, 11), (8193, 12), (12289, 12),
    (16385, 13), (24577, 13),
];

pub fn read_zlib(mut idat: IdatReader, buf: &mut [u8]) -> Result<IdatReader> {
    let cmf = idat.read_u8()?;
    println!("Compression method: {}", cmf & 0xF);
    if cmf & 0xF != 0x8 {
        return Err(Error::Format("Unrecognized compression method"));
    }
    println!("Compression window size: {}", 1 << ((cmf >> 4) as u32 + 8));
    if cmf >> 4 > 7 {
        warn!("Compression window size above 32K");
    }
    let flags = idat.read_u8()?;
    println!("Check bits: {:02X}", flags & 0x1F);
    if (((cmf as u16) << 8) + flags as u16) % 31 != 0 {
        warn!("Check bits are incorrect");
    }
    println!("Preset dictionary: {}", (flags & 0x20) >> 5);
    if flags & 0x20 != 0 {
        return Err(Error::Format("Preset dictionary set"));
    }
    println!("Compression level: {}", flags >> 6);
    let mut idat = BitReader::new(idat);
    let mut i = 0;
    loop {
        let (block_final, block_type) = next_block(&mut idat)?;
        match block_type {
            BlockType::Uncompressed(len) => {
                for _ in 0 .. len {
                    if i >= buf.len() {
                        return Err(Error::Format("Too much image data"));
                    }
                    buf[i] = idat.read_u8()?;
                    i += 1;
                }
            },
            BlockType::Huffman(literal_codes, distance_codes) => {
                loop {
                    let val = read_huffman(&mut idat, &literal_codes)?;
                    match val {
                        0 ..= 255 => {
                            if i >= buf.len() {
                                return Err(Error::Format("Too much image data"));
                            }
                            buf[i] = val as u8;
                            i += 1;
                        },
                        256 => break,
                        257 ..= 285 => {
                            let (base_length, length_extra_bits) = length_code_interpretation[(val - 257) as usize];
                            let length = base_length + idat.read_bits(length_extra_bits)? as usize;
                            let distance_code = read_huffman(&mut idat, &distance_codes)?;
                            if distance_code > 29 {
                                return Err(Error::Format("A distance code of 30-31 occured in the compressed data"));
                            }
                            let (base_distance, distance_extra_bits) = distance_code_interpretation[distance_code as usize];
                            let distance = base_distance + idat.read_bits(distance_extra_bits)? as usize;
                            if distance > i {
                                return Err(Error::Format("Distance refers past the beginning of the output"));
                            }
                            for _ in 0 .. length {
                                if i >= buf.len() {
                                    return Err(Error::Format("Too much image data"));
                                }
                                buf[i] = buf[i - distance];
                                i += 1;
                            }
                        },
                        _ => return Err(Error::Format("A value of 286-287 occured in the compressed data")),
                    }
                }
            },
        }
        if block_final {
            break;
        }
    }
    if i < buf.len() {
        return Err(Error::Format("Not enough image data"));
    }
    // TODO checksum
    idat.read_u32()?;
    Ok(idat.end())
}
