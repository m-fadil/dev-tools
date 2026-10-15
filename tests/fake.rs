mod common;
use common::dt;

#[test]
fn fixed_legacy_seeds_all_types_and_locales() {
    let cases: &[(&[&str], &str)] = &[
        (
            &["fake", "name", "--seed", "0", "--locale", "en", "-c", "5"],
            "Elizabeth Smith\nWilliam Jones\nThomas Jackson\nMatthew Thompson\nThomas Martin\n",
        ),
        (
            &["fake", "name", "--seed", "0", "--locale", "id", "-c", "5"],
            "Rina Santoso\nAni Kusuma\nRudi Halim\nNur Permana\nPutri Kurniawan\n",
        ),
        (
            &["fake", "firstname", "--seed", "0", "--locale", "en", "-c", "5"],
            "Elizabeth\nJames\nWilliam\nJennifer\nThomas\n",
        ),
        (
            &["fake", "firstname", "--seed", "0", "--locale", "id", "-c", "5"],
            "Rina\nBudi\nAni\nSri\nRudi\n",
        ),
        (
            &["fake", "lastname", "--seed", "0", "--locale", "en", "-c", "5"],
            "Martinez\nSmith\nDavis\nJones\nThomas\n",
        ),
        (
            &["fake", "lastname", "--seed", "0", "--locale", "id", "-c", "5"],
            "Wibowo\nSantoso\nSetiawan\nKusuma\nHutapea\n",
        ),
        (
            &["fake", "email", "--seed", "0", "--locale", "en", "-c", "5"],
            "elizabeth.smith15@example.com\nthomas.jackson@example.com\nlisa.thomas@example.com\nwilliam.brown@example.com\njoseph.harris@example.com\n",
        ),
        (
            &["fake", "email", "--seed", "0", "--locale", "id", "-c", "5"],
            "rina.santoso15@example.com\nrudi.halim@example.com\ndian.hutapea@example.com\njoko.hidayat@example.com\nbambang.maulana@example.com\n",
        ),
        (
            &["fake", "username", "--seed", "0", "--locale", "en", "-c", "5"],
            "elizabeth147\nthomas_jackson\nthomas98\nawilson660\nashley_perez\n",
        ),
        (
            &["fake", "username", "--seed", "0", "--locale", "id", "-c", "5"],
            "rina147\nrudi_halim\nputri98\nrsiregar660\nhadi_utama\n",
        ),
        (
            &["fake", "phone", "--seed", "0", "--locale", "en", "-c", "5"],
            "+1 411-555-0100\n+1 377-555-0114\n+1 569-555-0154\n+1 686-555-0164\n+1 560-555-0158\n",
        ),
        (
            &["fake", "phone", "--seed", "0", "--locale", "id", "-c", "5"],
            "+62 834-1002-3009\n+62 824-5205-5905\n+62 865-6840-5104\n+62 862-3216-1880\n+62 891-4673-7482\n",
        ),
        (
            &["fake", "company", "--seed", "0", "--locale", "en", "-c", "5"],
            "Smith Nusantara LLC\nThomas Initech Ltd\nThompson Pioneer Group\nRodriguez Pioneer Inc\nWilson Lumen Partners\n",
        ),
        (
            &["fake", "company", "--seed", "0", "--locale", "id", "-c", "5"],
            "PT Nusantara Santoso\nPT Initech Hutapea Tbk\nCV Pioneer Permana\nPT Pioneer Susanto\nPT Lumen Siregar Tbk\n",
        ),
        (
            &["fake", "address", "--seed", "0", "--locale", "en", "-c", "5"],
            "224 Cedar Street, New York\n545 Oak Street, Dallas\n456 Hill Street, Denver\n98 Lake Street, Phoenix\n720 Sunset Street, San Diego\n",
        ),
        (
            &["fake", "address", "--seed", "0", "--locale", "id", "-c", "5"],
            "Jl. Diponegoro No. 224, Jakarta\nJl. Sudirman No. 545, Tangerang\nJl. Ahmad Yani No. 456, Yogyakarta\nJl. Ahmad Yani No. 98, Semarang\nJl. Kartini No. 720, Depok\n",
        ),
        (
            &["fake", "city", "--seed", "0", "--locale", "en", "-c", "5"],
            "Phoenix\nNew York\nPhoenix\nChicago\nDallas\n",
        ),
        (
            &["fake", "city", "--seed", "0", "--locale", "id", "-c", "5"],
            "Semarang\nJakarta\nSemarang\nBandung\nTangerang\n",
        ),
        (
            &["fake", "country", "--seed", "0", "--locale", "en", "-c", "5"],
            "Australia\nUnited States\nUnited Kingdom\nCanada\nFrance\n",
        ),
        (
            &["fake", "country", "--seed", "0", "--locale", "id", "-c", "5"],
            "Thailand\nIndonesia\nSingapura\nMalaysia\nVietnam\n",
        ),
        (
            &["fake", "ip", "--seed", "0", "--locale", "en", "-c", "5"],
            "192.0.2.1\n192.0.2.38\n198.51.100.139\n198.51.100.165\n198.51.100.148\n",
        ),
        (
            &["fake", "ip", "--seed", "0", "--locale", "id", "-c", "5"],
            "192.0.2.1\n192.0.2.38\n198.51.100.139\n198.51.100.165\n198.51.100.148\n",
        ),
        (
            &["fake", "ipv6", "--seed", "0", "--locale", "en", "-c", "5"],
            "2001:db8:4434:15:3928:256d:77a2:8b88\n2001:db8:9d81:a623:74bc:94ca:3f0a:190c\n2001:db8:e747:6880:b865:a8ef:ce4c:a04e\n2001:db8:91d7:1f09:3def:d0bc:52d2:167c\n2001:db8:ca21:e871:8d3d:7803:5b2d:18f2\n",
        ),
        (
            &["fake", "ipv6", "--seed", "0", "--locale", "id", "-c", "5"],
            "2001:db8:4434:15:3928:256d:77a2:8b88\n2001:db8:9d81:a623:74bc:94ca:3f0a:190c\n2001:db8:e747:6880:b865:a8ef:ce4c:a04e\n2001:db8:91d7:1f09:3def:d0bc:52d2:167c\n2001:db8:ca21:e871:8d3d:7803:5b2d:18f2\n",
        ),
        (
            &["fake", "mac", "--seed", "0", "--locale", "en", "-c", "5"],
            "46:03:27:89:a7:93\n1a:e6:ba:ca:91:3d\n52:1c:e8:75:1c:83\n22:54:54:25:d6:7b\nc2:f8:86:63:a0:6f\n",
        ),
        (
            &["fake", "mac", "--seed", "0", "--locale", "id", "-c", "5"],
            "46:03:27:89:a7:93\n1a:e6:ba:ca:91:3d\n52:1c:e8:75:1c:83\n22:54:54:25:d6:7b\nc2:f8:86:63:a0:6f\n",
        ),
        (
            &["fake", "uuid", "--seed", "0", "--locale", "en", "-c", "5"],
            "4032789a-7931-46ba-8a91-3d51ce8751c8\n2545425d-67bc-4886-b3a0-6f771020c53f\n7d14bdc1-3750-4c8d-b9b3-4a7039934f92\na8576894-8c80-4a1d-bd0a-1becbd2ea3b8\nd451cdf7-a32f-4d8a-ba02-3c3151de7ccc\n",
        ),
        (
            &["fake", "uuid", "--seed", "0", "--locale", "id", "-c", "5"],
            "4032789a-7931-46ba-8a91-3d51ce8751c8\n2545425d-67bc-4886-b3a0-6f771020c53f\n7d14bdc1-3750-4c8d-b9b3-4a7039934f92\na8576894-8c80-4a1d-bd0a-1becbd2ea3b8\nd451cdf7-a32f-4d8a-ba02-3c3151de7ccc\n",
        ),
        (
            &[
                "fake",
                "date",
                "--seed",
                "0",
                "--locale",
                "en",
                "-c",
                "5",
                "--from",
                "2015-01-01",
                "--to",
                "2024-01-01",
            ],
            "2017-05-26\n2015-01-02\n2017-01-04\n2016-04-25\n2019-03-17\n",
        ),
        (
            &[
                "fake",
                "date",
                "--seed",
                "0",
                "--locale",
                "id",
                "-c",
                "5",
                "--from",
                "2015-01-01",
                "--to",
                "2024-01-01",
            ],
            "2017-05-26\n2015-01-02\n2017-01-04\n2016-04-25\n2019-03-17\n",
        ),
        (
            &[
                "fake",
                "datetime",
                "--seed",
                "0",
                "--locale",
                "en",
                "-c",
                "5",
                "--from",
                "2015-01-01",
                "--to",
                "2024-01-01",
            ],
            "2017-05-25T18:04:02Z\n2015-01-02T02:00:46Z\n2017-01-03T21:29:01Z\n2016-04-25T13:35:42Z\n2019-03-17T02:33:26Z\n",
        ),
        (
            &[
                "fake",
                "datetime",
                "--seed",
                "0",
                "--locale",
                "id",
                "-c",
                "5",
                "--from",
                "2015-01-01",
                "--to",
                "2024-01-01",
            ],
            "2017-05-25T18:04:02Z\n2015-01-02T02:00:46Z\n2017-01-03T21:29:01Z\n2016-04-25T13:35:42Z\n2019-03-17T02:33:26Z\n",
        ),
        (
            &["fake", "number", "--seed", "0", "--locale", "en", "-c", "5"],
            "266\n0\n223\n146\n467\n",
        ),
        (
            &["fake", "number", "--seed", "0", "--locale", "id", "-c", "5"],
            "266\n0\n223\n146\n467\n",
        ),
        (
            &["fake", "bool", "--seed", "0", "--locale", "en", "-c", "5"],
            "true\ntrue\ntrue\ntrue\ntrue\n",
        ),
        (
            &["fake", "bool", "--seed", "0", "--locale", "id", "-c", "5"],
            "true\ntrue\ntrue\ntrue\ntrue\n",
        ),
        (
            &["fake", "color", "--seed", "0", "--locale", "en", "-c", "5"],
            "#403278\n#9a7931\n#e6baca\n#913d51\n#ce8751\n",
        ),
        (
            &["fake", "color", "--seed", "0", "--locale", "id", "-c", "5"],
            "#403278\n#9a7931\n#e6baca\n#913d51\n#ce8751\n",
        ),
        (
            &["fake", "url", "--seed", "0", "--locale", "en", "-c", "5"],
            "https://www.example.com/lorem/2233\nhttps://www.example.com/est/5450\nhttps://www.example.org/similique/4560\nhttps://www.example.org/commodo/979\nhttps://www.example.net/culpa/7203\n",
        ),
        (
            &["fake", "url", "--seed", "0", "--locale", "id", "-c", "5"],
            "https://www.example.com/lorem/2233\nhttps://www.example.com/est/5450\nhttps://www.example.org/similique/4560\nhttps://www.example.org/commodo/979\nhttps://www.example.net/culpa/7203\n",
        ),
        (
            &["fake", "word", "--seed", "0", "--locale", "en", "-c", "5"],
            "duis\nlorem\naliquip\nenim\nest\n",
        ),
        (
            &["fake", "word", "--seed", "0", "--locale", "id", "-c", "5"],
            "duis\nlorem\naliquip\nenim\nest\n",
        ),
        (
            &["fake", "sentence", "--seed", "0", "--locale", "en", "-c", "5"],
            "Lorem aliquip enim est blanditiis molestias, similique.\nCulpa voluptatem mollitia illo occaecati, atque et ea inventore cillum tempor quae aut praesentium.\nQuis fugiat veritatis deserunt at doloremque rem.\nLaboris similique dolor non, sequi est vero eiusmod lorem.\nQuia est explicabo do commodo, accusantium veritatis ipsa incididunt nisi anim sint amet, veritatis ipsa.\n",
        ),
        (
            &["fake", "sentence", "--seed", "0", "--locale", "id", "-c", "5"],
            "Lorem aliquip enim est blanditiis molestias, similique.\nCulpa voluptatem mollitia illo occaecati, atque et ea inventore cillum tempor quae aut praesentium.\nQuis fugiat veritatis deserunt at doloremque rem.\nLaboris similique dolor non, sequi est vero eiusmod lorem.\nQuia est explicabo do commodo, accusantium veritatis ipsa incididunt nisi anim sint amet, veritatis ipsa.\n",
        ),
        (
            &["fake", "hex", "--seed", "0", "--locale", "en", "-c", "5"],
            "4032789a7931e6ba\nca913d51ce8751c8\n32545425d67bcf88\n663a06f771020c53\nfe7d14bdc13750dc\n",
        ),
        (
            &["fake", "hex", "--seed", "0", "--locale", "id", "-c", "5"],
            "4032789a7931e6ba\nca913d51ce8751c8\n32545425d67bcf88\n663a06f771020c53\nfe7d14bdc13750dc\n",
        ),
        (
            &["fake", "email", "--domain", "BÜCHER.test", "--seed", "0", "-c", "5"],
            "elizabeth.smith15@xn--bcher-kva.test\nthomas.jackson@xn--bcher-kva.test\nlisa.thomas@xn--bcher-kva.test\nwilliam.brown@xn--bcher-kva.test\njoseph.harris@xn--bcher-kva.test\n",
        ),
        (
            &["fake", "email", "--domain", "ＭＡＩＬ。TEST", "--seed", "0", "-c", "5"],
            "elizabeth.smith15@mail.test\nthomas.jackson@mail.test\nlisa.thomas@mail.test\nwilliam.brown@mail.test\njoseph.harris@mail.test\n",
        ),
        (
            &["fake", "number", "--min=-5", "--max", "5", "--seed", "0", "-c", "5"],
            "-3\n-5\n-3\n-4\n0\n",
        ),
        (
            &[
                "fake",
                "number",
                "--min",
                "9007194959773696",
                "--max",
                "9007199254740991",
                "--seed",
                "0",
                "-c",
                "5",
            ],
            "9007196104078434\n9007194961189943\n9007195918719752\n9007195587707140\n9007196966931412\n",
        ),
        (
            &["fake", "hex", "--length", "40", "--seed", "0", "-c", "5"],
            "4032789a7931e6baca913d51ce8751c832545425\nd67bcf88663a06f771020c53fe7d14bdc13750dc\n8d09b34a7039934f92ca85768948c80aa1d2d0a1\nbecbd2ea3b8cd451cdf7a32fdd8a1a023c3151de\n7cccd4892647af1583610d1635a36b2ea9456202\n",
        ),
        (
            &[
                "fake",
                "datetime",
                "--from",
                "2020-01-01T00:00:00Z",
                "--to",
                "2020-01-01T00:00:10Z",
                "--seed",
                "0",
                "-c",
                "5",
            ],
            "2020-01-01T00:00:02Z\n2020-01-01T00:00:00Z\n2020-01-01T00:00:02Z\n2020-01-01T00:00:01Z\n2020-01-01T00:00:05Z\n",
        ),
        (
            &["fake", "name", "--seed", "1", "--locale", "en", "-c", "5"],
            "Matthew Smith\nCharles King\nJoshua Martinez\nMatthew Harris\nJessica King\n",
        ),
        (
            &["fake", "name", "--seed", "1", "--locale", "id", "-c", "5"],
            "Nur Santoso\nFajar Salim\nTaufik Wibowo\nNur Maulana\nBambang Salim\n",
        ),
        (
            &["fake", "firstname", "--seed", "1", "--locale", "en", "-c", "5"],
            "Matthew\nJames\nCharles\nMichelle\nJoshua\n",
        ),
        (
            &["fake", "firstname", "--seed", "1", "--locale", "id", "-c", "5"],
            "Nur\nBudi\nFajar\nCitra\nTaufik\n",
        ),
        (
            &["fake", "lastname", "--seed", "1", "--locale", "en", "-c", "5"],
            "Perez\nSmith\nMoore\nKing\nAllen\n",
        ),
        (
            &["fake", "lastname", "--seed", "1", "--locale", "id", "-c", "5"],
            "Utama\nSantoso\nGunawan\nSalim\nTanjung\n",
        ),
        (
            &["fake", "email", "--seed", "1", "--locale", "en", "-c", "5"],
            "matthew.smith@example.com\nmichelle.allen61@example.com\nmargaret.anderson@example.com\nthomas.taylor40@example.com\nwilliam.garcia@example.com\n",
        ),
        (
            &["fake", "email", "--seed", "1", "--locale", "id", "-c", "5"],
            "nur.santoso@example.com\ncitra.tanjung61@example.com\nfitri.nasution@example.com\nputri.simanjuntak40@example.com\njoko.nugroho@example.com\n",
        ),
        (
            &["fake", "username", "--seed", "1", "--locale", "en", "-c", "5"],
            "matthew_smith\njoshua_martinez\njessica_king\njennifer155\nsarah_williams\n",
        ),
        (
            &["fake", "username", "--seed", "1", "--locale", "id", "-c", "5"],
            "nur_santoso\ntaufik_wibowo\nbambang_salim\neko155\nyuni_saputra\n",
        ),
        (
            &["fake", "phone", "--seed", "1", "--locale", "en", "-c", "5"],
            "+1 695-555-0100\n+1 617-555-0198\n+1 965-555-0128\n+1 684-555-0172\n+1 536-555-0199\n",
        ),
        (
            &["fake", "phone", "--seed", "1", "--locale", "id", "-c", "5"],
            "+62 866-1024-5747\n+62 898-9715-3529\n+62 865-7486-4832\n+62 899-5097-5399\n+62 823-4633-3227\n",
        ),
        (
            &["fake", "company", "--seed", "1", "--locale", "en", "-c", "5"],
            "Smith Zenith Ltd\nAllen Cascade Corp\nHarris Pioneer Group\nThomas Cascade Group\nWilson Initech LLC\n",
        ),
        (
            &["fake", "company", "--seed", "1", "--locale", "id", "-c", "5"],
            "PT Zenith Santoso Tbk\nCV Cascade Tanjung\nCV Pioneer Maulana\nCV Cascade Hutapea\nPT Initech Siregar\n",
        ),
        (
            &["fake", "address", "--seed", "1", "--locale", "en", "-c", "5"],
            "527 Hill Street, New York\n281 River Street, Nashville\n426 Hill Street, Boston\n489 River Street, Dallas\n248 Oak Street, San Diego\n",
        ),
        (
            &["fake", "address", "--seed", "1", "--locale", "id", "-c", "5"],
            "Jl. Pahlawan No. 527, Jakarta\nJl. Kartini No. 281, Manado\nJl. Ahmad Yani No. 426, Malang\nJl. Kartini No. 489, Tangerang\nJl. Sudirman No. 248, Depok\n",
        ),
        (
            &["fake", "city", "--seed", "1", "--locale", "en", "-c", "5"],
            "Denver\nNew York\nAustin\nNashville\nNashville\n",
        ),
        (
            &["fake", "city", "--seed", "1", "--locale", "id", "-c", "5"],
            "Yogyakarta\nJakarta\nBekasi\nManado\nManado\n",
        ),
        (
            &["fake", "country", "--seed", "1", "--locale", "en", "-c", "5"],
            "Brazil\nUnited States\nJapan\nItaly\nItaly\n",
        ),
        (
            &["fake", "country", "--seed", "1", "--locale", "id", "-c", "5"],
            "Australia\nIndonesia\nJepang\nBrasil\nBrasil\n",
        ),
        (
            &["fake", "ip", "--seed", "1", "--locale", "en", "-c", "5"],
            "198.51.100.1\n198.51.100.250\n203.0.113.72\n198.51.100.184\n198.51.100.253\n",
        ),
        (
            &["fake", "ip", "--seed", "1", "--locale", "id", "-c", "5"],
            "198.51.100.1\n198.51.100.250\n203.0.113.72\n198.51.100.184\n198.51.100.253\n",
        ),
        (
            &["fake", "ipv6", "--seed", "1", "--locale", "en", "-c", "5"],
            "2001:db8:a087:b3:8706:fb26:f7e7:47f6\n2001:db8:9ce3:b882:6d01:feac:748c:7d21\n2001:db8:2391:675b:3f5d:278c:7d38:113f\n2001:db8:6505:c461:492e:30dd:af0:6d68\n2001:db8:9723:cc76:4b7c:3499:a87b:8bcc\n",
        ),
        (
            &["fake", "ipv6", "--seed", "1", "--locale", "id", "-c", "5"],
            "2001:db8:a087:b3:8706:fb26:f7e7:47f6\n2001:db8:9ce3:b882:6d01:feac:748c:7d21\n2001:db8:2391:675b:3f5d:278c:7d38:113f\n2001:db8:6505:c461:492e:30dd:af0:6d68\n2001:db8:9723:cc76:4b7c:3499:a87b:8bcc\n",
        ),
        (
            &["fake", "mac", "--seed", "1", "--locale", "en", "-c", "5"],
            "a2:08:ff:49:b6:f7\n7e:26:32:71:6c:43\n0a:69:c4:3a:8c:3b\n76:cf:3c:25:63:2b\n92:24:09:d3:fb:64\n",
        ),
        (
            &["fake", "mac", "--seed", "1", "--locale", "id", "-c", "5"],
            "a2:08:ff:49:b6:f7\n7e:26:32:71:6c:43\n0a:69:c4:3a:8c:3b\n76:cf:3c:25:63:2b\n92:24:09:d3:fb:64\n",
        ),
        (
            &["fake", "uuid", "--seed", "1", "--locale", "en", "-c", "5"],
            "a08ff49b-6f77-4632-a16c-43069c43a8c3\n7cf3c256-32b9-4409-93fb-64afebede10d\ne8eb2af2-f261-45a8-91cd-424e42a85121\naa1883c4-ffd0-40f5-bd14-dcc312684e4e\naa36434e-f251-4c63-8d4e-86a538cc03aa\n",
        ),
        (
            &["fake", "uuid", "--seed", "1", "--locale", "id", "-c", "5"],
            "a08ff49b-6f77-4632-a16c-43069c43a8c3\n7cf3c256-32b9-4409-93fb-64afebede10d\ne8eb2af2-f261-45a8-91cd-424e42a85121\naa1883c4-ffd0-40f5-bd14-dcc312684e4e\naa36434e-f251-4c63-8d4e-86a538cc03aa\n",
        ),
        (
            &[
                "fake",
                "date",
                "--seed",
                "1",
                "--locale",
                "en",
                "-c",
                "5",
                "--from",
                "2015-01-01",
                "--to",
                "2024-01-01",
            ],
            "2020-08-23\n2015-01-09\n2019-10-01\n2023-10-31\n2023-09-20\n",
        ),
        (
            &[
                "fake",
                "date",
                "--seed",
                "1",
                "--locale",
                "id",
                "-c",
                "5",
                "--from",
                "2015-01-01",
                "--to",
                "2024-01-01",
            ],
            "2020-08-23\n2015-01-09\n2019-10-01\n2023-10-31\n2023-09-20\n",
        ),
        (
            &[
                "fake",
                "datetime",
                "--seed",
                "1",
                "--locale",
                "en",
                "-c",
                "5",
                "--from",
                "2015-01-01",
                "--to",
                "2024-01-01",
            ],
            "2020-08-23T04:36:33Z\n2015-01-09T23:48:56Z\n2019-09-30T17:14:32Z\n2023-10-30T17:08:56Z\n2023-09-19T01:23:45Z\n",
        ),
        (
            &[
                "fake",
                "datetime",
                "--seed",
                "1",
                "--locale",
                "id",
                "-c",
                "5",
                "--from",
                "2015-01-01",
                "--to",
                "2024-01-01",
            ],
            "2020-08-23T04:36:33Z\n2015-01-09T23:48:56Z\n2019-09-30T17:14:32Z\n2023-10-30T17:08:56Z\n2023-09-19T01:23:45Z\n",
        ),
        (
            &["fake", "number", "--seed", "1", "--locale", "en", "-c", "5"],
            "627\n2\n527\n982\n969\n",
        ),
        (
            &["fake", "number", "--seed", "1", "--locale", "id", "-c", "5"],
            "627\n2\n527\n982\n969\n",
        ),
        (
            &["fake", "bool", "--seed", "1", "--locale", "en", "-c", "5"],
            "false\ntrue\nfalse\nfalse\nfalse\n",
        ),
        (
            &["fake", "bool", "--seed", "1", "--locale", "id", "-c", "5"],
            "false\ntrue\nfalse\nfalse\nfalse\n",
        ),
        (
            &["fake", "color", "--seed", "1", "--locale", "en", "-c", "5"],
            "#a08ff4\n#9b6f77\n#263271\n#6c4306\n#9c43a8\n",
        ),
        (
            &["fake", "color", "--seed", "1", "--locale", "id", "-c", "5"],
            "#a08ff4\n#9b6f77\n#263271\n#6c4306\n#9c43a8\n",
        ),
        (
            &["fake", "url", "--seed", "1", "--locale", "en", "-c", "5"],
            "https://www.example.org/lorem/5274\nhttps://www.example.net/sequi/2811\nhttps://www.example.org/voluptatem/4258\nhttps://www.example.net/id/4888\nhttps://www.example.com/sunt/2475\n",
        ),
        (
            &["fake", "url", "--seed", "1", "--locale", "id", "-c", "5"],
            "https://www.example.org/lorem/5274\nhttps://www.example.net/sequi/2811\nhttps://www.example.org/voluptatem/4258\nhttps://www.example.net/id/4888\nhttps://www.example.com/sunt/2475\n",
        ),
        (
            &["fake", "word", "--seed", "1", "--locale", "en", "-c", "5"],
            "occaecati\nlorem\ndignissimos\nneque\nsequi\n",
        ),
        (
            &["fake", "word", "--seed", "1", "--locale", "id", "-c", "5"],
            "occaecati\nlorem\ndignissimos\nneque\nsequi\n",
        ),
        (
            &["fake", "sentence", "--seed", "1", "--locale", "en", "-c", "5"],
            "Lorem dignissimos neque sequi irure molestias voluptatem officia, quisquam id vero.\nIn nostrud consectetur officia quos ab reprehenderit ullamco mollitia blanditiis rem exercitation voluptatem.\nAtque dolore duis adipiscing excepturi explicabo ea porro voluptatem, sunt voluptate, unde.\nReprehenderit ipsam iusto consequuntur natus minim perspiciatis, dolores magna neque minim non et sunt.\nIn aliqua mollitia praesentium cillum et aliqua sed voluptate, perspiciatis similique et, iusto dignissimos.\n",
        ),
        (
            &["fake", "sentence", "--seed", "1", "--locale", "id", "-c", "5"],
            "Lorem dignissimos neque sequi irure molestias voluptatem officia, quisquam id vero.\nIn nostrud consectetur officia quos ab reprehenderit ullamco mollitia blanditiis rem exercitation voluptatem.\nAtque dolore duis adipiscing excepturi explicabo ea porro voluptatem, sunt voluptate, unde.\nReprehenderit ipsam iusto consequuntur natus minim perspiciatis, dolores magna neque minim non et sunt.\nIn aliqua mollitia praesentium cillum et aliqua sed voluptate, perspiciatis similique et, iusto dignissimos.\n",
        ),
        (
            &["fake", "hex", "--seed", "1", "--locale", "en", "-c", "5"],
            "a08ff49b6f772632\n716c43069c43a8c3\nb7cf3c25632b9240\n9d3fb64afebede10\nd4e8eb2af2f26165\n",
        ),
        (
            &["fake", "hex", "--seed", "1", "--locale", "id", "-c", "5"],
            "a08ff49b6f772632\n716c43069c43a8c3\nb7cf3c25632b9240\n9d3fb64afebede10\nd4e8eb2af2f26165\n",
        ),
        (
            &["fake", "email", "--domain", "BÜCHER.test", "--seed", "1", "-c", "5"],
            "matthew.smith@xn--bcher-kva.test\nmichelle.allen61@xn--bcher-kva.test\nmargaret.anderson@xn--bcher-kva.test\nthomas.taylor40@xn--bcher-kva.test\nwilliam.garcia@xn--bcher-kva.test\n",
        ),
        (
            &["fake", "email", "--domain", "ＭＡＩＬ。TEST", "--seed", "1", "-c", "5"],
            "matthew.smith@mail.test\nmichelle.allen61@mail.test\nmargaret.anderson@mail.test\nthomas.taylor40@mail.test\nwilliam.garcia@mail.test\n",
        ),
        (
            &["fake", "number", "--min=-5", "--max", "5", "--seed", "1", "-c", "5"],
            "1\n-5\n0\n5\n5\n",
        ),
        (
            &[
                "fake",
                "number",
                "--min",
                "9007194959773696",
                "--max",
                "9007199254740991",
                "--seed",
                "1",
                "-c",
                "5",
            ],
            "9007197653035763\n9007194971523529\n9007197225141483\n9007199173355517\n9007199118925099\n",
        ),
        (
            &["fake", "hex", "--length", "40", "--seed", "1", "-c", "5"],
            "a08ff49b6f772632716c43069c43a8c3b7cf3c25\n632b92409d3fb64afebede10d4e8eb2af2f26165\na881cd424e42a851214aa1883c4ffd020f5bd14d\ncc312684e4edaa36434ef2510c632d4e86a538cc\n03aa056da5590e89a11d2b4a48e112a086c41ff0\n",
        ),
        (
            &[
                "fake",
                "datetime",
                "--from",
                "2020-01-01T00:00:00Z",
                "--to",
                "2020-01-01T00:00:10Z",
                "--seed",
                "1",
                "-c",
                "5",
            ],
            "2020-01-01T00:00:06Z\n2020-01-01T00:00:00Z\n2020-01-01T00:00:05Z\n2020-01-01T00:00:10Z\n2020-01-01T00:00:10Z\n",
        ),
        (
            &["fake", "name", "--seed", "42", "--locale", "en", "-c", "5"],
            "Nancy Thomas\nPaul Thompson\nMichael Moore\nElizabeth Perez\nEmily Taylor\n",
        ),
        (
            &["fake", "name", "--seed", "42", "--locale", "id", "-c", "5"],
            "Nur Hutapea\nIntan Permana\nAndi Gunawan\nRina Utama\nBagus Simanjuntak\n",
        ),
        (
            &["fake", "firstname", "--seed", "42", "--locale", "en", "-c", "5"],
            "Nancy\nThomas\nPaul\nAnthony\nMichael\n",
        ),
        (
            &["fake", "firstname", "--seed", "42", "--locale", "id", "-c", "5"],
            "Nur\nPutri\nIntan\nRatna\nAndi\n",
        ),
        (
            &["fake", "lastname", "--seed", "42", "--locale", "en", "-c", "5"],
            "Lee\nThomas\nLewis\nThompson\nGarcia\n",
        ),
        (
            &["fake", "lastname", "--seed", "42", "--locale", "id", "-c", "5"],
            "Hartono\nHutapea\nPutra\nPermana\nNugroho\n",
        ),
        (
            &["fake", "email", "--seed", "42", "--locale", "en", "-c", "5"],
            "nancy.thomas@example.com\nanthony.garcia@example.com\nelizabeth.perez@example.com\nsarah.rodriguez@example.com\nmark.hernandez50@example.com\n",
        ),
        (
            &["fake", "email", "--seed", "42", "--locale", "id", "-c", "5"],
            "nur.hutapea@example.com\nratna.nugroho@example.com\nrina.utama@example.com\nrudi.susanto@example.com\narif.purnomo50@example.com\n",
        ),
        (
            &["fake", "username", "--seed", "42", "--locale", "en", "-c", "5"],
            "nthomas670\nmichael625\nemily882\nmark501\nanthony471\n",
        ),
        (
            &["fake", "username", "--seed", "42", "--locale", "id", "-c", "5"],
            "nhutapea670\nandi625\nbagus882\narif501\nwahyu471\n",
        ),
        (
            &["fake", "phone", "--seed", "42", "--locale", "en", "-c", "5"],
            "+1 675-555-0144\n+1 873-555-0166\n+1 338-555-0152\n+1 416-555-0162\n+1 883-555-0147\n",
        ),
        (
            &["fake", "phone", "--seed", "42", "--locale", "id", "-c", "5"],
            "+62 864-5034-8672\n+62 870-2573-5739\n+62 835-6622-8789\n+62 853-3249-8938\n+62 877-3763-2775\n",
        ),
        (
            &["fake", "company", "--seed", "42", "--locale", "en", "-c", "5"],
            "Thomas Pioneer Holdings\nGarcia Zenith Ltd\nPerez Nusantara Holdings\nRodriguez Quantum Labs\nHernandez Apex LLC\n",
        ),
        (
            &["fake", "company", "--seed", "42", "--locale", "id", "-c", "5"],
            "PT Pioneer Hutapea Persero\nPT Zenith Nugroho Tbk\nPT Nusantara Utama Persero\nPT Quantum Susanto Persero\nPT Apex Purnomo\n",
        ),
        (
            &["fake", "address", "--seed", "42", "--locale", "en", "-c", "5"],
            "852 Hill Street, Dallas\n527 Park Street, Houston\n865 Cedar Street, Denver\n882 Washington Street, Phoenix\n198 Park Street, Philadelphia\n",
        ),
        (
            &["fake", "address", "--seed", "42", "--locale", "id", "-c", "5"],
            "Jl. Ahmad Yani No. 852, Tangerang\nJl. Pahlawan No. 527, Medan\nJl. Diponegoro No. 865, Yogyakarta\nJl. Gatot Subroto No. 882, Semarang\nJl. Pahlawan No. 198, Makassar\n",
        ),
        (
            &["fake", "city", "--seed", "42", "--locale", "en", "-c", "5"],
            "Seattle\nDallas\nMiami\nBoston\nHouston\n",
        ),
        (
            &["fake", "city", "--seed", "42", "--locale", "id", "-c", "5"],
            "Bogor\nTangerang\nPontianak\nMalang\nMedan\n",
        ),
        (
            &["fake", "country", "--seed", "42", "--locale", "en", "-c", "5"],
            "Brazil\nFrance\nSpain\nIndia\nUnited Kingdom\n",
        ),
        (
            &["fake", "country", "--seed", "42", "--locale", "id", "-c", "5"],
            "Australia\nVietnam\nIndia\nJerman\nSingapura\n",
        ),
        (
            &["fake", "ip", "--seed", "42", "--locale", "en", "-c", "5"],
            "198.51.100.114\n203.0.113.171\n192.0.2.134\n192.0.2.159\n203.0.113.120\n",
        ),
        (
            &["fake", "ip", "--seed", "42", "--locale", "id", "-c", "5"],
            "198.51.100.114\n203.0.113.171\n192.0.2.134\n192.0.2.159\n203.0.113.120\n",
        ),
        (
            &["fake", "ipv6", "--seed", "42", "--locale", "en", "-c", "5"],
            "2001:db8:99e1:72c3:da3b:ab73:2cc0:86ce\n2001:db8:45f2:9fef:dd8f:78e9:3ffb:e1ce\n2001:db8:bee8:4e97:327f:802f:afc5:9c51\n2001:db8:fb:7885:d65b:d1c:97a2:812\n2001:db8:4457:fd0:2f89:c896:87c4:6f1\n",
        ),
        (
            &["fake", "ipv6", "--seed", "42", "--locale", "id", "-c", "5"],
            "2001:db8:99e1:72c3:da3b:ab73:2cc0:86ce\n2001:db8:45f2:9fef:dd8f:78e9:3ffb:e1ce\n2001:db8:bee8:4e97:327f:802f:afc5:9c51\n2001:db8:fb:7885:d65b:d1c:97a2:812\n2001:db8:4457:fd0:2f89:c896:87c4:6f1\n",
        ),
        (
            &["fake", "mac", "--seed", "42", "--locale", "en", "-c", "5"],
            "9a:7d:a2:84:9d:73\ne2:b4:38:a9:07:d0\n96:04:02:c8:02:d7\nce:57:00:89:3a:34\nbe:d8:34:41:aa:be\n",
        ),
        (
            &["fake", "mac", "--seed", "42", "--locale", "id", "-c", "5"],
            "9a:7d:a2:84:9d:73\ne2:b4:38:a9:07:d0\n96:04:02:c8:02:d7\nce:57:00:89:3a:34\nbe:d8:34:41:aa:be\n",
        ),
        (
            &["fake", "uuid", "--seed", "42", "--locale", "en", "-c", "5"],
            "97da2849-d73e-4438-9907-d090402c802d\nc5700893-a34b-4834-b1aa-be1f6f210579\n1be4b28d-1f9e-4959-8b47-dbfe6841b5ce\n7d2f2bb3-62a5-4ae5-a62d-50baee39720b\n44503f9f-2284-466a-95e5-79893545a32b\n",
        ),
        (
            &["fake", "uuid", "--seed", "42", "--locale", "id", "-c", "5"],
            "97da2849-d73e-4438-9907-d090402c802d\nc5700893-a34b-4834-b1aa-be1f6f210579\n1be4b28d-1f9e-4959-8b47-dbfe6841b5ce\n7d2f2bb3-62a5-4ae5-a62d-50baee39720b\n44503f9f-2284-466a-95e5-79893545a32b\n",
        ),
        (
            &[
                "fake",
                "date",
                "--seed",
                "42",
                "--locale",
                "en",
                "-c",
                "5",
                "--from",
                "2015-01-01",
                "--to",
                "2024-01-01",
            ],
            "2020-05-30\n2019-01-13\n2022-09-03\n2021-01-11\n2016-07-28\n",
        ),
        (
            &[
                "fake",
                "date",
                "--seed",
                "42",
                "--locale",
                "id",
                "-c",
                "5",
                "--from",
                "2015-01-01",
                "--to",
                "2024-01-01",
            ],
            "2020-05-30\n2019-01-13\n2022-09-03\n2021-01-11\n2016-07-28\n",
        ),
        (
            &[
                "fake",
                "datetime",
                "--seed",
                "42",
                "--locale",
                "en",
                "-c",
                "5",
                "--from",
                "2015-01-01",
                "--to",
                "2024-01-01",
            ],
            "2020-05-29T19:52:22Z\n2019-01-13T12:44:44Z\n2022-09-03T01:19:18Z\n2021-01-10T09:58:45Z\n2016-07-28T14:43:08Z\n",
        ),
        (
            &[
                "fake",
                "datetime",
                "--seed",
                "42",
                "--locale",
                "id",
                "-c",
                "5",
                "--from",
                "2015-01-01",
                "--to",
                "2024-01-01",
            ],
            "2020-05-29T19:52:22Z\n2019-01-13T12:44:44Z\n2022-09-03T01:19:18Z\n2021-01-10T09:58:45Z\n2016-07-28T14:43:08Z\n",
        ),
        (
            &["fake", "number", "--seed", "42", "--locale", "en", "-c", "5"],
            "601\n448\n853\n670\n174\n",
        ),
        (
            &["fake", "number", "--seed", "42", "--locale", "id", "-c", "5"],
            "601\n448\n853\n670\n174\n",
        ),
        (
            &["fake", "bool", "--seed", "42", "--locale", "en", "-c", "5"],
            "false\ntrue\nfalse\nfalse\ntrue\n",
        ),
        (
            &["fake", "bool", "--seed", "42", "--locale", "id", "-c", "5"],
            "false\ntrue\nfalse\nfalse\ntrue\n",
        ),
        (
            &["fake", "color", "--seed", "42", "--locale", "en", "-c", "5"],
            "#97da28\n#49d73e\n#b438a9\n#07d090\n#402c80\n",
        ),
        (
            &["fake", "color", "--seed", "42", "--locale", "id", "-c", "5"],
            "#97da28\n#49d73e\n#b438a9\n#07d090\n#402c80\n",
        ),
        (
            &["fake", "url", "--seed", "42", "--locale", "en", "-c", "5"],
            "https://www.example.org/anim/8524\nhttps://www.example.net/veniam/5266\nhttps://www.example.com/occaecati/8654\nhttps://www.example.org/commodo/8820\nhttps://www.example.net/voluptate/1973\n",
        ),
        (
            &["fake", "url", "--seed", "42", "--locale", "id", "-c", "5"],
            "https://www.example.org/anim/8524\nhttps://www.example.net/veniam/5266\nhttps://www.example.com/occaecati/8654\nhttps://www.example.org/commodo/8820\nhttps://www.example.net/voluptate/1973\n",
        ),
        (
            &["fake", "word", "--seed", "42", "--locale", "en", "-c", "5"],
            "quas\nanim\nvitae\nperspiciatis\nveniam\n",
        ),
        (
            &["fake", "word", "--seed", "42", "--locale", "id", "-c", "5"],
            "quas\nanim\nvitae\nperspiciatis\nveniam\n",
        ),
        (
            &["fake", "sentence", "--seed", "42", "--locale", "en", "-c", "5"],
            "Anim vitae perspiciatis veniam dignissimos aute occaecati explicabo laborum, commodo ipsam.\nArchitecto adipiscing dolores amet duis sed nostrud ipsa dignissimos, sit.\nVoluptatum dolores ea provident laboris.\nIusto ullamco irure, reprehenderit do, mollitia unde iste fugit tempor magni, deserunt magni magna.\nIrure natus quis odio veritatis et quisquam deleniti odit deserunt corrupti fugiat, dolores esse accusantium.\n",
        ),
        (
            &["fake", "sentence", "--seed", "42", "--locale", "id", "-c", "5"],
            "Anim vitae perspiciatis veniam dignissimos aute occaecati explicabo laborum, commodo ipsam.\nArchitecto adipiscing dolores amet duis sed nostrud ipsa dignissimos, sit.\nVoluptatum dolores ea provident laboris.\nIusto ullamco irure, reprehenderit do, mollitia unde iste fugit tempor magni, deserunt magni magna.\nIrure natus quis odio veritatis et quisquam deleniti odit deserunt corrupti fugiat, dolores esse accusantium.\n",
        ),
        (
            &["fake", "hex", "--seed", "42", "--locale", "en", "-c", "5"],
            "97da2849d73eb438\na907d090402c802d\n7c5700893a34bd83\n441aabe1f6f21057\n9e1be4b28d1f9e69\n",
        ),
        (
            &["fake", "hex", "--seed", "42", "--locale", "id", "-c", "5"],
            "97da2849d73eb438\na907d090402c802d\n7c5700893a34bd83\n441aabe1f6f21057\n9e1be4b28d1f9e69\n",
        ),
        (
            &["fake", "email", "--domain", "BÜCHER.test", "--seed", "42", "-c", "5"],
            "nancy.thomas@xn--bcher-kva.test\nanthony.garcia@xn--bcher-kva.test\nelizabeth.perez@xn--bcher-kva.test\nsarah.rodriguez@xn--bcher-kva.test\nmark.hernandez50@xn--bcher-kva.test\n",
        ),
        (
            &["fake", "email", "--domain", "ＭＡＩＬ。TEST", "--seed", "42", "-c", "5"],
            "nancy.thomas@mail.test\nanthony.garcia@mail.test\nelizabeth.perez@mail.test\nsarah.rodriguez@mail.test\nmark.hernandez50@mail.test\n",
        ),
        (
            &["fake", "number", "--min=-5", "--max", "5", "--seed", "42", "-c", "5"],
            "1\n-1\n4\n2\n-4\n",
        ),
        (
            &[
                "fake",
                "number",
                "--min",
                "9007194959773696",
                "--max",
                "9007199254740991",
                "--seed",
                "42",
                "-c",
                "5",
            ],
            "9007197541494652\n9007196885166986\n9007198621086400\n9007197836259501\n9007195710593674\n",
        ),
        (
            &["fake", "hex", "--length", "40", "--seed", "42", "-c", "5"],
            "97da2849d73eb438a907d090402c802d7c570089\n3a34bd83441aabe1f6f210579e1be4b28d1f9e69\n595b47dbfe6841b5ce17d2f2bb362a53ae5c62d5\n0baee39720bb44503f9f2284666a65e579893545\na32b51509376bec72d8de729c368f7ddb0128acf\n",
        ),
        (
            &[
                "fake",
                "datetime",
                "--from",
                "2020-01-01T00:00:00Z",
                "--to",
                "2020-01-01T00:00:10Z",
                "--seed",
                "42",
                "-c",
                "5",
            ],
            "2020-01-01T00:00:06Z\n2020-01-01T00:00:04Z\n2020-01-01T00:00:09Z\n2020-01-01T00:00:07Z\n2020-01-01T00:00:01Z\n",
        ),
        (
            &["fake", "name", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "Andrew Miller\nMargaret Allen\nPaul Jackson\nAnthony Taylor\nJohn King\n",
        ),
        (
            &["fake", "name", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "Rizki Pratama\nFitri Tanjung\nIntan Halim\nRatna Simanjuntak\nEko Salim\n",
        ),
        (
            &["fake", "firstname", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "Andrew\nMichael\nMargaret\nDonna\nPaul\n",
        ),
        (
            &["fake", "firstname", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "Rizki\nAndi\nFitri\nTaufik\nIntan\n",
        ),
        (
            &["fake", "lastname", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "Walker\nMiller\nHarris\nAllen\nLewis\n",
        ),
        (
            &["fake", "lastname", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "Harahap\nPratama\nMaulana\nTanjung\nPutra\n",
        ),
        (
            &["fake", "email", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "andrew.miller@example.com\ndonna.lewis@example.com\nanthony.taylor98@example.com\nemily.miller74@example.com\nsarah.sanchez@example.com\n",
        ),
        (
            &["fake", "email", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "rizki.pratama@example.com\ntaufik.putra@example.com\nratna.simanjuntak98@example.com\nbagus.pratama74@example.com\nrudi.ramadhan@example.com\n",
        ),
        (
            &["fake", "username", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "amiller944\npjackson476\njking190\nrichard_sanchez\nandrew392\n",
        ),
        (
            &["fake", "username", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "rpratama944\nihalim476\nesalim190\nhendra_ramadhan\nayu392\n",
        ),
        (
            &["fake", "phone", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "+1 908-555-0118\n+1 765-555-0194\n+1 867-555-0153\n+1 737-555-0147\n+1 308-555-0198\n",
        ),
        (
            &["fake", "phone", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "+62 890-2705-7440\n+62 895-8607-5852\n+62 871-5280-2222\n+62 898-8843-2704\n+62 841-7648-5283\n",
        ),
        (
            &["fake", "company", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "Miller Lumen Partners\nLewis Cascade Ltd\nTaylor Zenith LLC\nRobinson Cascade LLC\nSanchez Vertex Group\n",
        ),
        (
            &["fake", "company", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "PT Lumen Pratama Tbk\nPT Cascade Putra Tbk\nPT Zenith Simanjuntak\nPT Cascade Sihombing\nCV Vertex Ramadhan\n",
        ),
        (
            &["fake", "address", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "715 Sunset Street, Houston\n539 River Street, Miami\n136 Park Street, Dallas\n190 River Street, Miami\n476 Elm Street, Portland\n",
        ),
        (
            &["fake", "address", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "Jl. Kartini No. 715, Medan\nJl. Kartini No. 539, Pontianak\nJl. Pahlawan No. 136, Tangerang\nJl. Kartini No. 190, Pontianak\nJl. Diponegoro No. 476, Denpasar\n",
        ),
        (
            &["fake", "city", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "Detroit\nHouston\nBoston\nDetroit\nMiami\n",
        ),
        (
            &["fake", "city", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "Padang\nMedan\nMalang\nPadang\nPontianak\n",
        ),
        (
            &["fake", "country", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "Spain\nUnited Kingdom\nIndia\nItaly\nSpain\n",
        ),
        (
            &["fake", "country", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "India\nSingapura\nJerman\nBrasil\nIndia\n",
        ),
        (
            &["fake", "ip", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "203.0.113.49\n203.0.113.240\n203.0.113.137\n203.0.113.121\n192.0.2.252\n",
        ),
        (
            &["fake", "ip", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "203.0.113.49\n203.0.113.240\n203.0.113.137\n203.0.113.121\n192.0.2.252\n",
        ),
        (
            &["fake", "ipv6", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "2001:db8:e57b:3081:b735:f1ad:d861:8a05\n2001:db8:ae35:79bf:22c7:fd0a:df18:3079\n2001:db8:57fb:bd1a:79da:c1b7:ea79:1a21\n2001:db8:24c3:6465:eb94:ec82:caa8:da20\n2001:db8:1d85:175c:83dd:4672:4ec2:50c\n",
        ),
        (
            &["fake", "ipv6", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "2001:db8:e57b:3081:b735:f1ad:d861:8a05\n2001:db8:ae35:79bf:22c7:fd0a:df18:3079\n2001:db8:57fb:bd1a:79da:c1b7:ea79:1a21\n2001:db8:24c3:6465:eb94:ec82:caa8:da20\n2001:db8:1d85:175c:83dd:4672:4ec2:50c\n",
        ),
        (
            &["fake", "mac", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "e6:3b:fd:8a:72:fd\n32:5b:7c:e1:26:ee\nca:d1:18:44:07:48\n6e:36:89:24:ec:04\n4a:ba:c2:fe:f9:66\n",
        ),
        (
            &["fake", "mac", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "e6:3b:fd:8a:72:fd\n32:5b:7c:e1:26:ee\nca:d1:18:44:07:48\n6e:36:89:24:ec:04\n4a:ba:c2:fe:f9:66\n",
        ),
        (
            &["fake", "uuid", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "e3bfd8a7-2fd3-4b7c-a126-eecd11844074\n6368924e-c044-4ac2-aef9-668e496e2e30\n89b37c48-f567-4ee6-a773-3e42d3192259\nce343f6d-68bc-49b5-8d2b-36711868316b\ndd2b7804-ed0b-411e-b53b-1e90666eac49\n",
        ),
        (
            &["fake", "uuid", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "e3bfd8a7-2fd3-4b7c-a126-eecd11844074\n6368924e-c044-4ac2-aef9-668e496e2e30\n89b37c48-f567-4ee6-a773-3e42d3192259\nce343f6d-68bc-49b5-8d2b-36711868316b\ndd2b7804-ed0b-411e-b53b-1e90666eac49\n",
        ),
        (
            &[
                "fake",
                "date",
                "--seed",
                "4294967295",
                "--locale",
                "en",
                "-c",
                "5",
                "--from",
                "2015-01-01",
                "--to",
                "2024-01-01",
            ],
            "2023-01-26\n2016-09-15\n2021-06-11\n2023-07-02\n2022-08-11\n",
        ),
        (
            &[
                "fake",
                "date",
                "--seed",
                "4294967295",
                "--locale",
                "id",
                "-c",
                "5",
                "--from",
                "2015-01-01",
                "--to",
                "2024-01-01",
            ],
            "2023-01-26\n2016-09-15\n2021-06-11\n2023-07-02\n2022-08-11\n",
        ),
        (
            &[
                "fake",
                "datetime",
                "--seed",
                "4294967295",
                "--locale",
                "en",
                "-c",
                "5",
                "--from",
                "2015-01-01",
                "--to",
                "2024-01-01",
            ],
            "2023-01-25T12:59:14Z\n2016-09-14T19:33:38Z\n2021-06-10T08:24:31Z\n2023-07-01T02:59:54Z\n2022-08-10T07:00:42Z\n",
        ),
        (
            &[
                "fake",
                "datetime",
                "--seed",
                "4294967295",
                "--locale",
                "id",
                "-c",
                "5",
                "--from",
                "2015-01-01",
                "--to",
                "2024-01-01",
            ],
            "2023-01-25T12:59:14Z\n2016-09-14T19:33:38Z\n2021-06-10T08:24:31Z\n2023-07-01T02:59:54Z\n2022-08-10T07:00:42Z\n",
        ),
        (
            &["fake", "number", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "897\n189\n716\n945\n846\n",
        ),
        (
            &["fake", "number", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "897\n189\n716\n945\n846\n",
        ),
        (
            &["fake", "bool", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "false\ntrue\nfalse\nfalse\nfalse\n",
        ),
        (
            &["fake", "bool", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "false\ntrue\nfalse\nfalse\nfalse\n",
        ),
        (
            &["fake", "color", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "#e3bfd8\n#a72fd3\n#5b7ce1\n#26eecd\n#118440\n",
        ),
        (
            &["fake", "color", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "#e3bfd8\n#a72fd3\n#5b7ce1\n#26eecd\n#118440\n",
        ),
        (
            &["fake", "url", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "https://www.example.net/nostrud/7156\nhttps://www.example.net/beatae/5391\nhttps://www.example.net/laborum/1359\nhttps://www.example.net/nemo/1894\nhttps://www.example.org/laudantium/4760\n",
        ),
        (
            &["fake", "url", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "https://www.example.net/nostrud/7156\nhttps://www.example.net/beatae/5391\nhttps://www.example.net/laborum/1359\nhttps://www.example.net/nemo/1894\nhttps://www.example.org/laudantium/4760\n",
        ),
        (
            &["fake", "word", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "voluptas\nnostrud\nvoluptatem\nmagni\nbeatae\n",
        ),
        (
            &["fake", "word", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "voluptas\nnostrud\nvoluptatem\nmagni\nbeatae\n",
        ),
        (
            &["fake", "sentence", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "Nostrud voluptatem magni beatae blanditiis, unde laborum magna porro nemo nostrud nulla laudantium, laborum.\nIusto aute voluptate dolor id commodo.\nBlanditiis quas enim duis quia ab sit aute reprehenderit.\nExcepturi officia non dignissimos aspernatur commodo dolores sunt voluptas minim ipsam nisi amet perspiciatis deleniti.\nVoluptas qui similique id at laboris ex consequuntur duis ad nemo aliquip eiusmod quas.\n",
        ),
        (
            &["fake", "sentence", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "Nostrud voluptatem magni beatae blanditiis, unde laborum magna porro nemo nostrud nulla laudantium, laborum.\nIusto aute voluptate dolor id commodo.\nBlanditiis quas enim duis quia ab sit aute reprehenderit.\nExcepturi officia non dignissimos aspernatur commodo dolores sunt voluptas minim ipsam nisi amet perspiciatis deleniti.\nVoluptas qui similique id at laboris ex consequuntur duis ad nemo aliquip eiusmod quas.\n",
        ),
        (
            &["fake", "hex", "--seed", "4294967295", "--locale", "en", "-c", "5"],
            "e3bfd8a72fd35b7c\ne126eecd11844074\n86368924ec044bac\n2fef9668e496e2e3\n0a89b37c48f567ce\n",
        ),
        (
            &["fake", "hex", "--seed", "4294967295", "--locale", "id", "-c", "5"],
            "e3bfd8a72fd35b7c\ne126eecd11844074\n86368924ec044bac\n2fef9668e496e2e3\n0a89b37c48f567ce\n",
        ),
        (
            &[
                "fake",
                "email",
                "--domain",
                "BÜCHER.test",
                "--seed",
                "4294967295",
                "-c",
                "5",
            ],
            "andrew.miller@xn--bcher-kva.test\ndonna.lewis@xn--bcher-kva.test\nanthony.taylor98@xn--bcher-kva.test\nemily.miller74@xn--bcher-kva.test\nsarah.sanchez@xn--bcher-kva.test\n",
        ),
        (
            &[
                "fake",
                "email",
                "--domain",
                "ＭＡＩＬ。TEST",
                "--seed",
                "4294967295",
                "-c",
                "5",
            ],
            "andrew.miller@mail.test\ndonna.lewis@mail.test\nanthony.taylor98@mail.test\nemily.miller74@mail.test\nsarah.sanchez@mail.test\n",
        ),
        (
            &[
                "fake",
                "number",
                "--min=-5",
                "--max",
                "5",
                "--seed",
                "4294967295",
                "-c",
                "5",
            ],
            "4\n-3\n2\n5\n4\n",
        ),
        (
            &[
                "fake",
                "number",
                "--min",
                "9007194959773696",
                "--max",
                "9007199254740991",
                "--seed",
                "4294967295",
                "-c",
                "5",
            ],
            "9007198809879507\n9007195773576612\n9007198033478544\n9007199014480132\n9007198590036527\n",
        ),
        (
            &["fake", "hex", "--length", "40", "--seed", "4294967295", "-c", "5"],
            "e3bfd8a72fd35b7ce126eecd1184407486368924\nec044bac2fef9668e496e2e30a89b37c48f567ce\ne6a7733e42d31922598ce343f6d68bc79b50d2b3\n6711868316b2dd2b7804ed0b511ef53b1e90666e\nac49e472c9d837cda5f18470470bec41a6a3d1dd\n",
        ),
        (
            &[
                "fake",
                "datetime",
                "--from",
                "2020-01-01T00:00:00Z",
                "--to",
                "2020-01-01T00:00:10Z",
                "--seed",
                "4294967295",
                "-c",
                "5",
            ],
            "2020-01-01T00:00:09Z\n2020-01-01T00:00:02Z\n2020-01-01T00:00:07Z\n2020-01-01T00:00:10Z\n2020-01-01T00:00:09Z\n",
        ),
    ];
    for (args, expected) in cases {
        let r = dt(args);
        assert_eq!(r.code, 0, "{args:?}: {}", r.err);
        assert_eq!(&r.out, expected, "{args:?}");
    }
}

fn lines(extra: &[&str]) -> Vec<String> {
    let mut args = vec!["fake"];
    args.extend_from_slice(extra);
    let r = dt(&args);
    assert_eq!(r.code, 0, "{}", r.err);
    r.out.lines().map(str::to_owned).collect()
}
fn pattern(s: &str, p: &str) {
    assert!(regex::Regex::new(p).unwrap().is_match(s), "{s}");
}
#[test]
fn all_types_reproducible_and_listing() {
    let types = "name firstname lastname email username phone company address city country ip ipv6 mac uuid date datetime number bool color url word sentence hex";
    let listing = dt(&["fake"]);
    assert_eq!(listing.code, 0);
    for kind in types.split_whitespace() {
        assert!(listing.out.contains(&format!("  {kind}\n")));
        for locale in ["en", "id"] {
            let mut args = vec![kind, "-c", "5", "--seed", "9", "--locale", locale];
            if kind.starts_with("date") {
                args.extend(["--from", "2015-01-01", "--to", "2024-01-01"]);
            }
            let a = lines(&args);
            assert_eq!(a.len(), 5);
            assert_eq!(a, lines(&args));
        }
        if kind != "email" {
            let r = dt(&["fake", kind, "--domain", "example.com"]);
            assert_eq!(r.code, 2);
            assert!(r.out.is_empty());
        }
    }
    let r = dt(&["fake", "nope"]);
    assert_eq!(r.code, 2);
    assert!(r.err.contains("hex"));
}
#[test]
fn domains_and_emails() {
    for locale in ["en", "id"] {
        for s in lines(&["email", "-c", "20", "--seed", "1", "--locale", locale]) {
            pattern(&s, r"^[a-z]+\.[a-z]+\d*@example\.com$");
        }
    }
    let defaults = lines(&["email", "-c", "10", "--seed", "42"]);
    let custom = lines(&["email", "--domain", "Mail.Perusahaan.TEST", "-c", "10", "--seed", "42"]);
    assert_eq!(
        custom,
        defaults
            .iter()
            .map(|s| s.replace("@example.com", "@mail.perusahaan.test"))
            .collect::<Vec<_>>()
    );
    let long = "a".repeat(64) + ".test";
    let too_long = ["a".repeat(63), "b".repeat(63), "c".repeat(63), "d".repeat(63)].join(".");
    for domain in [
        "",
        "localhost",
        "user@example.com",
        "@example.com",
        "https://example.com",
        "example.com:443",
        "example.com/path",
        "example.com\\path",
        "example.com?x=1",
        "example.com#fragment",
        "example%2ecom",
        " example.com",
        "example .com",
        "a..test",
        ".example.com",
        "example.com.",
        "-a.test",
        "a-.test",
        "a_b.test",
        "[::1]",
        &long,
        &too_long,
    ] {
        let r = dt(&["fake", "email", "--domain", domain]);
        assert_eq!(r.code, 2, "{domain}");
        assert!(r.out.is_empty());
    }
    for args in [
        vec!["fake", "--domain", "example.com"],
        vec!["fake", "name", "--domain", "example.com", "-h"],
        vec!["fake", "unknown", "-h"],
        vec!["fake", "email", "extra", "-h"],
    ] {
        let r = dt(&args);
        assert_eq!(r.code, 2);
        assert!(r.out.is_empty());
    }
    let r = dt(&[
        "fake",
        "email",
        "--domain",
        "BÜCHER.test",
        "--seed",
        "42",
        "-c",
        "3",
        "--json",
    ]);
    let j: serde_json::Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(j["type"], "email");
    assert_eq!(j["values"].as_array().unwrap().len(), 3);
    for v in j["values"].as_array().unwrap() {
        assert!(v.as_str().unwrap().ends_with("@xn--bcher-kva.test"));
    }
}
#[test]
fn documented_formats() {
    for (kind, extra, p) in [
        ("phone", vec![], r"^\+1 \d{3}-555-01\d{2}$"),
        ("phone", vec!["--locale", "id"], r"^\+62 8\d{2}-\d{4}-\d{4}$"),
        ("ip", vec![], r"^(192\.0\.2|198\.51\.100|203\.0\.113)\.\d{1,3}$"),
        ("ipv6", vec![], r"^2001:db8(:[0-9a-f]{1,4}){6}$"),
        ("mac", vec![], r"^([0-9a-f]{2}:){5}[0-9a-f]{2}$"),
        (
            "uuid",
            vec![],
            r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$",
        ),
        ("color", vec![], r"^#[0-9a-f]{6}$"),
        ("hex", vec!["--length", "40"], r"^[0-9a-f]{40}$"),
        ("bool", vec![], r"^(true|false)$"),
        ("url", vec![], r"^https://www\.example\.(com|org|net)/"),
    ] {
        let mut args = vec![kind, "-c", "60", "--seed", "3"];
        args.extend(extra);
        for s in lines(&args) {
            pattern(&s, p);
            if kind == "mac" {
                assert_eq!(u8::from_str_radix(&s[..2], 16).unwrap() & 3, 2);
            }
        }
    }
    pattern(&lines(&["hex"])[0], r"^[0-9a-f]{16}$");
}
#[test]
fn numbers_dates_and_json() {
    for s in lines(&["number", "-c", "200", "--min=-5", "--max", "5", "--seed", "5"]) {
        assert!((-5..=5).contains(&s.parse::<i64>().unwrap()));
    }
    assert_eq!(
        lines(&["number", "-c", "3", "--min", "7", "--max", "7"]),
        ["7", "7", "7"]
    );
    assert_eq!(lines(&["number", "--min", "5000", "-c", "3"]), ["5000", "5000", "5000"]);
    assert_eq!(lines(&["number", "--min", "9007199254740991"]), ["9007199254740991"]);
    for (min, max) in [("0", "4294967295"), ("9007194959773696", "9007199254740991")] {
        for s in lines(&["number", "--min", min, "--max", max, "--seed", "5", "-c", "30"]) {
            assert!((min.parse::<i64>().unwrap()..=max.parse::<i64>().unwrap()).contains(&s.parse::<i64>().unwrap()));
        }
    }
    for s in lines(&[
        "date",
        "-c",
        "100",
        "--from",
        "2020-02-28",
        "--to",
        "2020-03-01",
        "--seed",
        "6",
    ]) {
        assert!(["2020-02-28", "2020-02-29", "2020-03-01"].contains(&s.as_str()));
    }
    for s in lines(&[
        "datetime",
        "-c",
        "50",
        "--from",
        "2020-01-01T00:00:00Z",
        "--to",
        "2020-01-01T00:00:10Z",
        "--seed",
        "6",
    ]) {
        pattern(&s, r"^2020-01-01T00:00:(0\d|10)Z$");
    }
    pattern(&lines(&["date"])[0], r"^\d{4}-\d{2}-\d{2}$");
    for (kind, locale, count) in [("number", "en", "3"), ("bool", "id", "2")] {
        let r = dt(&["fake", kind, "--locale", locale, "-c", count, "--seed", "1", "--json"]);
        let j: serde_json::Value = serde_json::from_str(&r.out).unwrap();
        assert_eq!(
            j.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(),
            ["type", "locale", "values"]
        );
        assert_eq!(j["locale"], locale);
        assert_eq!(j["values"].as_array().unwrap().len(), count.parse::<usize>().unwrap());
        for v in j["values"].as_array().unwrap() {
            assert!(if kind == "number" {
                v.is_number()
            } else {
                v.is_boolean()
            });
        }
    }
}
#[test]
fn invalid_options() {
    for extra in [
        vec!["number", "--min", "9", "--max", "1"],
        vec!["number", "--min", "x"],
        vec!["number", "--max", "4294967296"],
        vec!["number", "--max", "9007199254740991"],
        vec!["number", "--min=-2147483648", "--max", "2147483648"],
        vec!["date", "--from", "2020-02-30"],
        vec!["date", "--from", "abc"],
        vec!["date", "--from", "2021-01-01", "--to", "2020-01-01"],
        vec!["name", "-c", "0"],
        vec!["name", "-c", "1001"],
        vec!["name", "--locale", "fr"],
        vec!["name", "--min", "1"],
        vec!["name", "--length", "4"],
        vec!["name", "extra"],
        vec!["name", "--from", "2020-01-01"],
        vec!["hex", "--length", "0"],
        vec!["name", "--seed", "x"],
    ] {
        let mut args = vec!["fake"];
        args.extend(extra);
        let r = dt(&args);
        assert_eq!(r.code, 2, "{args:?}");
        assert!(r.out.is_empty());
    }
    for kind in ["date", "datetime"] {
        for opt in ["--from", "--to"] {
            let r = dt(&["fake", kind, opt, "2020-01-01T00:00:00.125Z"]);
            assert_eq!(r.code, 2);
            assert!(r.out.is_empty());
            assert!(r.err.contains("ISO 8601"));
        }
    }
}

#[test]
fn email_domain_whatwg_ipv4() {
    for (d, want) in [("127.1", "127.0.0.1"), ("0x7f.1", "127.0.0.1"), ("1.2.3", "1.2.0.3")] {
        let r = dt(&["fake", "email", "--domain", d, "-c", "1", "--seed", "1"]);
        assert_eq!(r.code, 0, "{d}: {}", r.err);
        assert!(r.out.trim_end().ends_with(&format!("@{want}")), "{d}: {}", r.out);
    }
    for d in ["foo.1", "example.123"] {
        let r = dt(&["fake", "email", "--domain", d]);
        assert_eq!(r.code, 2, "{d}");
        assert!(r.err.contains("Domain email tidak valid"), "{d}: {}", r.err);
    }
}
