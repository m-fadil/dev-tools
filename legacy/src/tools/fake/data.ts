export const LOCALES = ['id', 'en'] as const;
export type Locale = (typeof LOCALES)[number];

export interface LocaleData {
    firstNames: string[];
    lastNames: string[];
    cities: string[];
    streets: string[];
    companySuffixes: string[];
}

const split = (s: string) => s.split(' ');

export const DATA: Record<Locale, LocaleData> = {
    en: {
        firstNames: split(
            'James Mary Robert Patricia John Jennifer Michael Linda William Elizabeth David Barbara Richard Susan ' +
                'Joseph Jessica Thomas Sarah Charles Karen Daniel Nancy Matthew Lisa Anthony Margaret Mark Sandra ' +
                'Steven Ashley Paul Emily Andrew Donna Joshua Michelle',
        ),
        lastNames: split(
            'Smith Johnson Williams Brown Jones Garcia Miller Davis Rodriguez Martinez Hernandez Lopez Gonzalez Wilson ' +
                'Anderson Thomas Taylor Moore Jackson Martin Lee Perez Thompson White Harris Sanchez Clark Ramirez ' +
                'Lewis Robinson Walker Young Allen King',
        ),
        cities: [
            'New York',
            'Los Angeles',
            'Chicago',
            'Houston',
            'Phoenix',
            'Philadelphia',
            'San Antonio',
            'San Diego',
            'Dallas',
            'Austin',
            'Seattle',
            'Denver',
            'Boston',
            'Portland',
            'Atlanta',
            'Miami',
            'Detroit',
            'Nashville',
        ],
        streets: [
            'Main',
            'Oak',
            'Maple',
            'Cedar',
            'Elm',
            'Washington',
            'Lake',
            'Hill',
            'Park',
            'Pine',
            'Sunset',
            'River',
        ],
        companySuffixes: ['Inc', 'LLC', 'Corp', 'Group', 'Ltd', 'Partners', 'Holdings', 'Labs'],
    },
    id: {
        firstNames: split(
            'Budi Siti Agus Dewi Eko Sri Andi Ani Joko Rina Dedi Wati Hendra Lestari Bambang Putri Rudi Yuni Fajar ' +
                'Indah Ahmad Nur Dian Ratna Wahyu Fitri Arif Maya Hadi Intan Bagus Rizki Ayu Taufik Citra',
        ),
        lastNames: split(
            'Santoso Wijaya Saputra Hidayat Kusuma Nugroho Pratama Setiawan Susanto Wibowo Purnomo Rahayu Lestari ' +
                'Siregar Nasution Hutapea Simanjuntak Gunawan Halim Kurniawan Hartono Utama Permana Firmansyah ' +
                'Maulana Ramadhan Suryani Handayani Putra Sihombing Harahap Lubis Tanjung Salim',
        ),
        cities: [
            'Jakarta',
            'Surabaya',
            'Bandung',
            'Medan',
            'Semarang',
            'Makassar',
            'Palembang',
            'Depok',
            'Tangerang',
            'Bekasi',
            'Bogor',
            'Yogyakarta',
            'Malang',
            'Denpasar',
            'Balikpapan',
            'Pontianak',
            'Padang',
            'Manado',
        ],
        streets: ['Merdeka', 'Sudirman', 'Diponegoro', 'Gatot Subroto', 'Ahmad Yani', 'Pahlawan', 'Veteran', 'Kartini'],
        companySuffixes: ['PT', 'CV', 'Tbk', 'Persero'],
    },
};

export const COUNTRIES: Record<Locale, string[]> = {
    en: [
        'United States',
        'Canada',
        'United Kingdom',
        'Australia',
        'Germany',
        'France',
        'Japan',
        'Brazil',
        'India',
        'Mexico',
        'Spain',
        'Italy',
    ],
    id: [
        'Indonesia',
        'Malaysia',
        'Singapura',
        'Thailand',
        'Filipina',
        'Vietnam',
        'Jepang',
        'Australia',
        'Jerman',
        'Prancis',
        'India',
        'Brasil',
    ],
};

export const COMPANY_WORDS = [
    'Acme',
    'Globex',
    'Initech',
    'Umbra',
    'Nusantara',
    'Vertex',
    'Nimbus',
    'Quantum',
    'Solaris',
    'Pioneer',
    'Zenith',
    'Apex',
    'Harbor',
    'Summit',
    'Lumen',
    'Cascade',
];

export const TLDS = ['com', 'org', 'net'];
