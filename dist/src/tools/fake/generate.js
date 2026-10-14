import { COMPANY_WORDS, COUNTRIES, DATA, EMAIL_DOMAINS, TLDS } from './data.js';
import { pick, randInt } from './rng.js';
import { generate as lorem } from '../lorem/text.js';
export const TYPES = [
    'name',
    'firstname',
    'lastname',
    'email',
    'username',
    'phone',
    'company',
    'address',
    'city',
    'country',
    'ip',
    'ipv6',
    'mac',
    'uuid',
    'date',
    'datetime',
    'number',
    'bool',
    'color',
    'url',
    'word',
    'sentence',
    'hex',
];
const DAY = 86_400_000;
const hex = (rng, digits) => Array.from({ length: digits }, () => randInt(rng, 0, 15).toString(16)).join('');
const fullName = (p) => [pick(p.rng, DATA[p.locale].firstNames), pick(p.rng, DATA[p.locale].lastNames)];
const slug = (s) => s.toLowerCase().replace(/[^a-z0-9]+/g, '');
function uuid(rng) {
    const h = hex(rng, 32).split('');
    h[12] = '4';
    h[16] = '89ab'[randInt(rng, 0, 3)];
    const s = h.join('');
    return `${s.slice(0, 8)}-${s.slice(8, 12)}-${s.slice(12, 16)}-${s.slice(16, 20)}-${s.slice(20)}`;
}
function company(p) {
    const d = DATA[p.locale];
    const word = pick(p.rng, COMPANY_WORDS);
    const last = pick(p.rng, d.lastNames);
    const suffix = pick(p.rng, d.companySuffixes);
    if (suffix === 'PT' || suffix === 'CV')
        return `${suffix} ${word} ${last}`;
    return p.locale === 'id' ? `PT ${word} ${last} ${suffix}` : `${last} ${word} ${suffix}`;
}
const makers = {
    name: (p) => fullName(p).join(' '),
    firstname: (p) => pick(p.rng, DATA[p.locale].firstNames),
    lastname: (p) => pick(p.rng, DATA[p.locale].lastNames),
    email: (p) => {
        const [f, l] = fullName(p);
        const n = p.rng() < 0.4 ? String(randInt(p.rng, 1, 99)) : '';
        return `${slug(f)}.${slug(l)}${n}@${pick(p.rng, EMAIL_DOMAINS)}`;
    },
    username: (p) => {
        const [f, l] = fullName(p);
        const style = randInt(p.rng, 0, 2);
        const n = randInt(p.rng, 1, 999);
        return style === 0 ? `${slug(f)}${n}` : style === 1 ? `${slug(f)}_${slug(l)}` : `${slug(f)[0]}${slug(l)}${n}`;
    },
    phone: (p) => {
        // id: +62 8xx mobile-style format only (may match real subscribers); en: NANP fictional range 555-0100..0199
        if (p.locale === 'id') {
            return `+62 8${randInt(p.rng, 11, 99)}-${randInt(p.rng, 1000, 9999)}-${randInt(p.rng, 1000, 9999)}`;
        }
        return `+1 ${randInt(p.rng, 201, 989)}-555-01${String(randInt(p.rng, 0, 99)).padStart(2, '0')}`;
    },
    company,
    address: (p) => {
        const d = DATA[p.locale];
        const street = pick(p.rng, d.streets);
        const city = pick(p.rng, d.cities);
        const no = randInt(p.rng, 1, 999);
        return p.locale === 'id' ? `Jl. ${street} No. ${no}, ${city}` : `${no} ${street} Street, ${city}`;
    },
    city: (p) => pick(p.rng, DATA[p.locale].cities),
    country: (p) => pick(p.rng, COUNTRIES[p.locale]),
    ip: (p) => {
        const net = pick(p.rng, ['192.0.2', '198.51.100', '203.0.113']);
        return `${net}.${randInt(p.rng, 1, 254)}`;
    },
    ipv6: (p) => `2001:db8:${Array.from({ length: 6 }, () => randInt(p.rng, 0, 0xffff).toString(16)).join(':')}`,
    mac: (p) => {
        // locally administered, unicast
        const first = (randInt(p.rng, 0, 63) * 4 + 2).toString(16).padStart(2, '0');
        return [first, ...Array.from({ length: 5 }, () => hex(p.rng, 2))].join(':');
    },
    uuid: (p) => uuid(p.rng),
    date: (p) => {
        const day = randInt(p.rng, Math.floor(p.from / DAY), Math.floor(p.to / DAY));
        return new Date(day * DAY).toISOString().slice(0, 10);
    },
    datetime: (p) => {
        const sec = randInt(p.rng, Math.ceil(p.from / 1000), Math.floor(p.to / 1000));
        return `${new Date(sec * 1000).toISOString().slice(0, 19)}Z`;
    },
    number: (p) => randInt(p.rng, p.min, p.max),
    bool: (p) => p.rng() < 0.5,
    color: (p) => `#${hex(p.rng, 6)}`,
    url: (p) => `https://www.example.${pick(p.rng, TLDS)}/${lorem(p.rng, 'words', 1, false)}/${randInt(p.rng, 1, 9999)}`,
    word: (p) => lorem(p.rng, 'words', 1, false),
    sentence: (p) => lorem(p.rng, 'sentences', 1, false),
    hex: (p) => hex(p.rng, p.length),
};
export function make(type, p) {
    return makers[type](p);
}
//# sourceMappingURL=generate.js.map