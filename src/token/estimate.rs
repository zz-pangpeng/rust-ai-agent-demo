/// 估算文本的 token 数量。
///
/// 该函数是一个启发式估算器，通过对字符分类后按经验系数折算：
/// - ASCII 字符：约 3.6 字符 / token（代码、标点密集时更碎）
/// - CJK 字符及标点：约 1.16 token / 字符
/// - 其他（emoji、非拉丁文字等）：约 2 字符 / token
///
/// 采用整数分数通分计算，避免浮点精度问题，结果与浮点版本完全一致。
pub fn estimate_tokens(text: &str) -> usize {
    let mut ascii: usize = 0;
    let mut cjk: usize = 0;
    let mut other: usize = 0;

    for ch in text.chars() {
        let c = ch as u32;
        if (0x4e00..=0x9fff).contains(&c) {
            // 常用汉字
            cjk += 1;
        } else if (0x3000..=0x303f).contains(&c) {
            // CJK 标点
            cjk += 1;
        } else if c < 0x80 {
            // ASCII
            ascii += 1;
        } else {
            // emoji 等
            other += 1;
        }
    }

    // 系数（倒数）：
    //   ascii : 1 / 3.6  =   5 / 18
    //   cjk   : 1 / 1.16 =  25 / 29
    //   other : 1 / 2.0  =   1 / 2
    //
    // 通分到最小公倍数分母 522（= 18 * 29）：
    //   ascii : 145 / 522
    //   cjk   : 450 / 522
    //   other : 261 / 522
    const DENOM: usize = 522;              // 18 * 29
    const NUM_ASCII: usize = 145;          // 5  * 29
    const NUM_CJK: usize = 450;            // 25 * 18
    const NUM_OTHER: usize = 261;          // 522 / 2

    // 加权总和（分子，未除以 DENOM）
    let weighted = ascii * NUM_ASCII + cjk * NUM_CJK + other * NUM_OTHER;

    // 向上取整：ceil(a / b) == (a + b - 1) / b （整数除法）
    (weighted + DENOM - 1) / DENOM
}