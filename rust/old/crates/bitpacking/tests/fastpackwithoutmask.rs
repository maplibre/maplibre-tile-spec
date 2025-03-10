use bitpacking::BitPacking;

static TEST_DATA: [u32; 32] = [
    1506, 468, 3129, 2824, 1715, 3459, 448, 1685, 242, 3189, 1405, 1689, 2603, 1459, 2860, 2397,
    4019, 823, 464, 123, 2422, 1142, 1492, 3915, 2152, 2890, 662, 2045, 3823, 739, 3650, 326
];

#[test]
fn fastpackwithoutmask0() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [0; 32];

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 0);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask1() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        -2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 
        0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 1);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask2() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        -14, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 
        0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 2);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask3() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        -16414, -4194305, -133, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 
        0, 0, 0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 3);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask4() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        -557859358, -8388622, -8912909, 1862270696, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 4);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask5() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        1874132962, -33622280, -268702746, 2071854973, -1242562567, 0, 0, 0, 0, 0, 0, 0, 0, 0, 
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 5);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask6() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        -1837598, -570794144, -135333001, -134361101, -84050595, -54591491, 0, 0, 0, 0, 0, 0, 0, 
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 6);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask7() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        1930358754, -13669253, -1158447238, -1615610179, 1941401460, -1515356521, -1121976577, 0, 
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 7);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask8() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        205116898, -1781692493, -1652722190, 1596832555, 2077441971, 1339326326, -6337944, 
        1313009647, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 8);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask9() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        1894231522, -240096392, -51711158, -1226656547, 1874308811, -681320634, 1873147278, 
        -1417572, -1550787346, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 
        0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 9);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask10() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        -1013492254, 906933186, -722229924, 732321777, 1475532494, -586358861, 1373206046, 
        678154973, -268474003, 1373907854, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 
        0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 10);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask11() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        240035298, -1041549805, -221075706, 861889448, -1294359875, -1078776916, 1626764313, 
        2001877911, -631739927, -285214043, 687409521, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 11);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask12() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        958219746, 917745804, 1766965464, 2110214386, 975923605, -1780798373, -801931341, 
        1769342897, -189410233, -1766545304, 1055883218, 342770222, 0, 0, 0, 0, 0, 0, 0, 
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 12);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask13() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        -465926686, 1798669360, -1469048058, -1902054860, -1286826507, -888445278, 
        -273462550, 1032274022, 149722976, 1752848757, -1973851832, -957418498, 171151493, 
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 13);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask14() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        -1871378974, -1288953661, 469983430, 1089608276, 1683477277, 1825188634, 628404929, 
        13488051, 1979837469, 1564548489, -2006434516, -198614318, -1194397921, 85517344, 0, 
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 
        0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 14);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask15() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        1089078754, 828441358, 7084139, -234018297, 1598438016, -1565469919, 749743512, 
        -1884089670, 1618215323, -1332256753, -1776857053, -1526175714, -6249083, 387510000, 
        42744072, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 15);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask16() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        30672354, 185076793, 226690739, 110428608, 208994546, 110691709, 95619627, 157092652, 
        53940147, 8061392, 74844534, 256574932, 189401192, 134021782, 48434927, 21368386, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 16);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask17() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        61343202, 1480601828, -1335858384, 1249931265, -369036797, -938085352, 1621274676, 
        -2100625226, 263390382, 121636462, -1755315240, 1962970816, 1745331585, 1477874696, 
        -264247286, -2141429522, 10683280, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 17);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask18() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        122684898, -1040137328, 201765634, 1075576886, 15860133, 1473262036, 721528384, 
        -1072247798, 39272626, 215748531, 515906816, -670468608, -1067630575, 141034450, 
        694168872, -285081792, 537627662, 5341412, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 
        0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 18);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask19() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        245368290, 268635712, -2140458986, 117442241, -234827104, 1080272896, 221380959, 
        -645881168, -1607684094, 263389483, 1946163640, 1610675712, 37421207, -379578544, 
        1342728193, 10846298, -286257158, 134312320, 2670649, 0, 0, 0, 0, 0, 0, 0, 0, 0, 
        0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 19);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask20() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        490735074, -2146682624, 112394416, -1073686480, 6901761, -951058190, -1878688512, 
        170590313, 738220848, 9818123, 862982067, -1342058496, 158728199, -738179232, 
        16035845, -1264580504, -805136896, 250544255, 1107308080, 1335310, 0, 0, 0, 0, 0, 
        0, 0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 20);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask21() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        981468642, 3204096, 1798309252, 1771008, 883425392, -1610550784, 368312718, 
        -1341961088, 191234210, -402470144, 263389258, 1073768160, 4030471, -335505568, 
        24444936, 1744861784, 23674888, -25163176, 15659011, -1870658106, 667651, 0, 
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 21);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask22() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        1962935778, 12816384, -1291834336, 56672262, 1409293312, 15859738, -805102272, 
        442761303, -1073075456, -1296039572, 2454528, -843051085, 1900544, 1979711980, 
        18710537, 738221376, 141033533, 1610797696, 536084521, -1072763136, -467664712, 
        333824, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 22);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask23() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        -369097246, 51265536, 805396736, 1813512299, 458752, -234877654, 104497152, 
        536960832, -1565523757, 2988032, -1174393680, 263389202, 105344, 257949812, 
        9920512, 1342186416, 513146903, 550912, -1518336603, 16752640, 402714352, 
        956825623, 166912, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 23);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask24() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        -738195998, 205062145, 722944, -2097150285, 29360141, 431360, 1962934514, 92078092, 
        432384, -1291843029, 187432965, 613632, 922750899, 30408707, 31488, 1979713910, 
        97779716, 1002240, 1241516136, 43384843, 523520, -486535441, 239206402, 83456, 0, 0, 
        0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 24);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask25() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        -1476393502, 820248579, 5783552, 1610640176, 1879048624, 55214080, 61952, -201320214, 
        885522453, 10661888, 46688, -1367342389, 263389188, 421376, -671086784, -1755316221, 
        9355264, -2147388160, 1744832421, 378798088, 677888, -268419096, 1549795566, 59801600, 
        41728, 0, 0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 25);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask26() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        1342178786, -1013972985, 46268416, 439040, 13836, -1522532324, 15859713, 3265536, 
        1073764304, 721420710, 382468106, 11714560, 153408, -603975757, 486539276, 2015232, 
        620032, 1073746392, -759168931, 141033475, 2959360, 1073752416, -285212161, 
        193724430, 14950400, 20864, 0, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 26);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask27() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        -1610611230, 239075342, 370147331, 7024640, 442752, -1610610944, -234880814, 
        1671954432, 23019520, 864768, -2147442000, -1342176551, 731906092, 263389185, 
        1685504, 29696, 1610612982, 989855895, 391118850, 32071680, 550912, -2147460528, 
        -100663131, -286261233, 24215552, 3737600, 10432, 0, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 27);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask28() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        1073743330, 956301341, -1333788660, 112394240, 14168064, 114688, 26960, 1342177522, 
        2097152199, 1771044869, 170590208, 5976064, 732160, 38352, 1879052211, -805306317, 
        128974849, 158728192, 4677632, 381952, 62640, -1610610584, -1778384716, 2144337922, 
        250544128, 3026944, 934400, 5216, 0, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 28);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask29() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        -2147482142, -469761990, -2080374736, 1798307845, 453378048, 7340032, 3450880, 61952, 
        102048, -2147478028, -1342176436, 1711276194, -889192437, 1256718338, 263389184, 
        6742016, 475136, 15744, 38752, 2284, 1476395381, 1744830586, 1765801992, 173539329, 
        67010560, 15659008, 378368, 233600, 2608, 0, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 29);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask30() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        1506, -1879048075, 536871107, -1291845588, 1623195654, 469762051, 441712640, 
        15859712, 52248576, 5754880, 1729536, 666368, 93376, 45760, 9588, -1073737805, 
        205, -335544291, 1979711489, 494927881, 1564475393, 1026293760, 141033472, 
        47349760, 2711552, 2094080, 978688, 47296, 58400, 1304, 0, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 30);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask31() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        1506, 1073742058, 782, 805306721, 402653291, 108, 704643079, -234881011, 981467136, 
        1598029830, -752877567, -1565523968, 764936192, 749731840, 314179584, 263389184, 
        26968064, 7602176, 1007616, 9920512, 2338816, 1527808, 2004480, 550912, 369920, 
        42368, 65440, 61168, 5912, 14600, 652, 0
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 31);

    assert_eq!(output, expected);
}

#[test]
fn fastpackwithoutmask32() {
    let mut output: [u32; 32] = [0; 32];
    let expected: [u32; 32] = [
        1506, 468, 3129, 2824, 1715, 3459, 448, 1685, 242, 3189, 1405, 1689, 2603, 1459, 
        2860, 2397, 4019, 823, 464, 123, 2422, 1142, 1492, 3915, 2152, 2890, 662, 2045, 
        3823, 739, 3650, 326
    ].map(|i| i as u32);

    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 32);

    assert_eq!(output, expected);
    assert_eq!(output, TEST_DATA); // Equals
}

#[test]
#[should_panic]
fn fastpackwithoutmask33() {
    let mut output: [u32; 32] = [0; 32];

    // 33 bitwidth is not supported, please panic
    BitPacking::fastpackwithoutmask(&TEST_DATA, &mut output, 33);
}
