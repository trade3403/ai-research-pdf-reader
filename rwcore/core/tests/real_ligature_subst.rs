//! Independently extracted real direct GSUB type 4 references from original SFNT bytes.
use rwcore::*;
type Rule=(u16,&'static [u16]);
type Set=(u16,&'static [Rule]);
type Case=(usize,usize,usize,&'static [Set]);
fn check(font:&[u8],glyph_count:u32,lookup_count:usize,cases:&[Case]){
    let c=parse_font(font,FontLimits::default()).unwrap();
    let bytes=c.face(0).unwrap().table(*b"GSUB").unwrap();
    let layout=parse_gsub_layout(bytes,GsubLimits::default()).unwrap();
    assert_eq!(layout.lookups.len(),lookup_count);
    let mut seen=0;
    for (i,lookup) in layout.lookups.iter().enumerate(){
        if lookup.kind!=4{continue;}
        for (j,&offset) in lookup.subtables.iter().enumerate(){
            let &(ei,ej,eo,sets)=&cases[seen];
            assert_eq!((i,j,offset),(ei,ej,eo));
            let r=parse_ligature_substitution(&bytes[offset..],glyph_count,LigatureLimits::default()).unwrap();
            assert_eq!(r.len(),sets.len());
            for id in 0..glyph_count as u16{
                let expected=sets.binary_search_by_key(&id,|s|s.0).ok().map(|z|sets[z].1);
                match expected{
                    None=>assert!(r.rules(id).is_none()),
                    Some(rules)=>{
                        let actual=r.rules(id).unwrap();
                        assert_eq!(actual.len(),rules.len());
                        for (a,&(out,tail)) in actual.iter().zip(rules){
                            assert_eq!((a.output,a.remaining.as_slice()),(out,tail));
                            let mut input=vec![id];input.extend_from_slice(tail);
                            // Font order, including a higher-priority shorter prefix.
                            let &(want,used)=rules.iter().find(|rule|input[1..].starts_with(rule.1)).unwrap();
                            let m=LigatureMatch{output:want,consumed:used.len()+1};
                            assert_eq!(r.match_prefix(&input),Some(m));
                            let mut run=vec![0];run.extend_from_slice(&input);run.push(1);
                            assert_eq!(r.replace_at(&mut run,1).unwrap(),Some(m));
                            let mut expected_run=vec![0,want];expected_run.extend_from_slice(&input[m.consumed..]);expected_run.push(1);
                            assert_eq!(run,expected_run);
                            for n in 1..input.len(){
                                let prefix=&input[..n];
                                let expected=rules.iter().find(|rule|prefix[1..].starts_with(rule.1))
                                    .map(|rule|LigatureMatch{output:rule.0,consumed:rule.1.len()+1});
                                assert_eq!(r.match_prefix(prefix),expected);
                            }
                        }
                    }
                }
            }
            seen+=1;
        }
    }
    assert_eq!(seen,cases.len());
}
#[test]fn real_regular_ligatures(){
    let cases:&[Case]=&[
        (8,0,622,&[
            (103,&[(104,&[172])]),
        ]),
        (9,0,654,&[
            (32,&[(33,&[172])]),
        ]),
        (11,0,710,&[
            (142,&[(157,&[176,145]),(156,&[176,143])]),
            (144,&[(158,&[176,145])]),
        ]),
    ];
    check(include_bytes!("fixtures/abeezee/ABeeZee-Regular.ttf"),268,15,cases);
}
#[test]fn real_italic_ligatures(){
    let cases:&[Case]=&[
        (8,0,622,&[
            (103,&[(104,&[172])]),
        ]),
        (9,0,654,&[
            (32,&[(33,&[172])]),
        ]),
        (11,0,710,&[
            (142,&[(157,&[176,145]),(156,&[176,143])]),
            (144,&[(158,&[176,145])]),
        ]),
    ];
    check(include_bytes!("fixtures/abeezee/ABeeZee-Italic.ttf"),268,15,cases);
}
#[test]fn real_aref_ligatures(){
    let cases:&[Case]=&[
        (75,0,5464,&[
            (24,&[(25,&[123])]),
        ]),
    ];
    check(include_bytes!("fixtures/arefruqaa/ArefRuqaa-Regular.ttf"),739,86,cases);
}
#[test]fn real_alegreya_ligatures(){
    let cases:&[Case]=&[
        (4,0,6084,&[
            (1975,&[(1976,&[1982]),(1977,&[1983]),(1978,&[1987]),(1979,&[1995])]),
            (1980,&[(1981,&[1995])]),
            (1989,&[(1990,&[1983])]),
            (1991,&[(1992,&[1975]),(1993,&[1983]),(1994,&[1995])]),
            (1995,&[(1996,&[1982]),(1997,&[1983])]),
            (2011,&[(2012,&[2017]),(2013,&[2018]),(2014,&[2021])]),
            (2015,&[(2016,&[2028])]),
            (2024,&[(2025,&[2011]),(2026,&[2018]),(2027,&[2028])]),
            (2028,&[(2029,&[2017]),(2030,&[2018])]),
        ]),
        (5,0,6286,&[
            (1975,&[(1976,&[1982]),(1977,&[1983]),(1978,&[1987]),(1979,&[1995])]),
            (1980,&[(1981,&[1995])]),
            (1989,&[(1990,&[1983])]),
            (1991,&[(1992,&[1975]),(1993,&[1983]),(1994,&[1995])]),
            (1995,&[(1996,&[1982]),(1997,&[1983])]),
            (2011,&[(2012,&[2017]),(2013,&[2018]),(2014,&[2021])]),
            (2015,&[(2016,&[2028])]),
            (2024,&[(2025,&[2011]),(2026,&[2018]),(2027,&[2028])]),
            (2028,&[(2029,&[2017]),(2030,&[2018])]),
        ]),
        (6,0,6488,&[
            (1986,&[(2096,&[1982]),(2095,&[1983]),(2098,&[1991]),(2097,&[1998])]),
            (1988,&[(2092,&[1982]),(2091,&[1983]),(2094,&[1991]),(2093,&[1998])]),
        ]),
        (7,0,6566,&[
            (1986,&[(2096,&[1982]),(2095,&[1983]),(2098,&[1991]),(2097,&[1998])]),
            (1988,&[(2092,&[1982]),(2091,&[1983]),(2094,&[1991]),(2093,&[1998])]),
        ]),
        (8,0,6644,&[
            (85,&[(245,&[100])]),
            (337,&[(500,&[353])]),
        ]),
        (31,0,7154,&[
            (116,&[(1965,&[380,1807])]),
        ]),
        (40,0,9498,&[
            (1912,&[(69,&[47]),(2100,&[71]),(99,&[83]),(221,&[195]),(239,&[230]),(320,&[298]),(2104,&[323]),(352,&[335]),(474,&[448]),(492,&[483]),(582,&[560]),(2106,&[587]),(617,&[599]),(738,&[712]),(756,&[747])]),
            (1991,&[(69,&[47]),(2100,&[71]),(99,&[83]),(221,&[195]),(239,&[230]),(320,&[298]),(2104,&[323]),(352,&[335]),(474,&[448]),(492,&[483]),(582,&[560]),(2106,&[587]),(617,&[599]),(738,&[712]),(756,&[747])]),
            (2046,&[(231,&[230]),(484,&[483]),(748,&[747])]),
            (2051,&[(90,&[83]),(138,&[127]),(342,&[335]),(391,&[380]),(607,&[599]),(656,&[645])]),
            (2058,&[(69,&[47]),(2100,&[71]),(99,&[83]),(221,&[195]),(239,&[230]),(320,&[298]),(2104,&[323]),(352,&[335]),(474,&[448]),(492,&[483]),(582,&[560]),(2106,&[587]),(617,&[599]),(738,&[712]),(756,&[747])]),
        ]),
        (42,0,10010,&[
            (322,&[(511,&[335]),(510,&[353]),(512,&[359])]),
        ]),
        (46,0,10720,&[
            (330,&[(2102,&[440])]),
        ]),
        (47,0,10742,&[
            (1835,&[(1927,&[1966,1906]),(1933,&[1966,1907]),(1928,&[1906]),(1926,&[1966])]),
            (1904,&[(1943,&[1906]),(1942,&[1966])]),
            (1906,&[(1929,&[1966,1835])]),
            (1907,&[(1931,&[1966,1835]),(1932,&[1835]),(1945,&[1904])]),
            (1966,&[(1930,&[1835]),(1944,&[1904])]),
        ]),
        (50,0,14500,&[
            (104,&[(109,&[1798])]),
            (359,&[(363,&[1798])]),
        ]),
    ];
    check(include_bytes!("fixtures/alegreyasans/AlegreyaSans-Regular.ttf"),2109,57,cases);
}
