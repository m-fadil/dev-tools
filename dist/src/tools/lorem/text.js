import { pick, randInt } from '../fake/rng.js';
export const WORDS = ('lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna ' +
    'aliqua enim ad minim veniam quis nostrud exercitation ullamco laboris nisi aliquip ex ea commodo consequat duis ' +
    'aute irure in reprehenderit voluptate velit esse cillum fugiat nulla pariatur excepteur sint occaecat cupidatat ' +
    'non proident sunt culpa qui officia deserunt mollit anim id est laborum at vero eos accusamus iusto odio ' +
    'dignissimos ducimus blanditiis praesentium voluptatum deleniti atque corrupti quos dolores quas molestias ' +
    'excepturi occaecati cupiditate provident similique mollitia animi perspiciatis unde omnis iste natus error ' +
    'voluptatem accusantium doloremque laudantium totam rem aperiam eaque ipsa quae ab illo inventore veritatis ' +
    'quasi architecto beatae vitae dicta explicabo nemo ipsam quia voluptas aspernatur aut odit fugit consequuntur ' +
    'magni dolores ratione sequi nesciunt neque porro quisquam').split(' ');
export const UNITS = ['words', 'sentences', 'paragraphs'];
export const LIMITS = { words: 5000, sentences: 500, paragraphs: 100 };
export const CLASSIC_OPENING = ['lorem', 'ipsum', 'dolor', 'sit', 'amet,', 'consectetur', 'adipiscing', 'elit'];
function capitalize(word) {
    return word.charAt(0).toUpperCase() + word.slice(1);
}
function randomWords(rng, n) {
    const out = [];
    for (let i = 0; i < n; i++) {
        let word = pick(rng, WORDS);
        // avoid immediate repetition
        while (word === out[i - 1])
            word = pick(rng, WORDS);
        out.push(word);
    }
    return out;
}
/** 5-15 words, capitalised, optional commas in the middle, ending with a period. */
function sentence(rng, opening) {
    const length = randInt(rng, 5, 15);
    const words = opening ? [...opening] : [];
    for (const word of randomWords(rng, Math.max(0, length - words.length)))
        words.push(word);
    for (let i = opening?.length ?? 0; i < words.length - 1; i++) {
        if (i >= 2 && !words[i].endsWith(',') && rng() < 0.12)
            words[i] += ',';
        if (words[i].endsWith(',') && i + 1 < words.length)
            i++;
    }
    return `${capitalize(words.join(' '))}.`;
}
function paragraph(rng, opening) {
    const count = randInt(rng, 3, 7);
    return Array.from({ length: count }, (_, i) => sentence(rng, i === 0 ? opening : undefined)).join(' ');
}
export function generate(rng, unit, count, classic) {
    if (unit === 'words') {
        const words = classic ? [...CLASSIC_OPENING] : [];
        for (const word of randomWords(rng, Math.max(0, count - words.length)))
            words.push(word);
        return words.slice(0, count).join(' ');
    }
    if (unit === 'sentences') {
        return Array.from({ length: count }, (_, i) => sentence(rng, classic && i === 0 ? CLASSIC_OPENING : undefined)).join(' ');
    }
    return Array.from({ length: count }, (_, i) => paragraph(rng, classic && i === 0 ? CLASSIC_OPENING : undefined)).join('\n\n');
}
//# sourceMappingURL=text.js.map