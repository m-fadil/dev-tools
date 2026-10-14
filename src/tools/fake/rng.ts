import { randomBytes } from 'node:crypto';
import { integer } from '../../io.js';

export type Rng = () => number;

/** mulberry32: small seeded PRNG returning floats in [0, 1). */
export function mulberry32(seed: number): Rng {
    let a = seed >>> 0;
    return () => {
        a = (a + 0x6d2b79f5) >>> 0;
        let t = a;
        t = Math.imul(t ^ (t >>> 15), t | 1);
        t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
}

/** RNG from the --seed option (0-4294967295); without it the seed comes from node:crypto. */
export function rngFromSeed(seed: string | undefined): Rng {
    const n = seed === undefined ? randomBytes(4).readUInt32LE() : integer(seed, 0, 0, 4294967295, 'seed');
    return mulberry32(n);
}

/** Uniform integer in [min, max]; callers must keep max - min below 2^32 (the rng has 32 bits). */
export function randInt(rng: Rng, min: number, max: number): number {
    return min + Math.floor(rng() * (max - min + 1));
}

export function pick<T>(rng: Rng, items: readonly T[]): T {
    return items[Math.floor(rng() * items.length)] as T;
}
