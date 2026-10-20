use std::{
    env,
    error::Error,
    fs::File,
    io::{BufRead, BufReader},
};

const HEADER: &str = "ts_ns,bid_price,bid_qty,ask_price,ask_qty";

#[derive(Debug)]
struct Quote {
    ts_ns: u64,
    bid_price: i64,
    bid_qty: u64,
    ask_price: i64,
    ask_qty: u64,
}

fn parse_quote(text: &str) -> Result<Quote, Box<dyn Error>> {
    let fields: Vec<&str> = text.split(',').map(str::trim).collect();

    if fields.len() != 5 {
        return Err("需要恰好 5 列".into());
    }

    let quote = Quote {
        ts_ns: fields[0]
            .parse()
            .map_err(|e| format!("ts_ns 无效：{e}"))?,
        bid_price: fields[1]
            .parse()
            .map_err(|e| format!("bid_price 无效：{e}"))?,
        bid_qty: fields[2]
            .parse()
            .map_err(|e| format!("bid_qty 无效：{e}"))?,
        ask_price: fields[3]
            .parse()
            .map_err(|e| format!("ask_price 无效：{e}"))?,
        ask_qty: fields[4]
            .parse()
            .map_err(|e| format!("ask_qty 无效：{e}"))?,
    };

    if quote.bid_price <= 0 || quote.ask_price <= quote.bid_price {
        return Err("报价必须满足：0 < 买价 < 卖价".into());
    }

    if quote.bid_qty == 0 || quote.ask_qty == 0 {
        return Err("买卖数量必须大于 0".into());
    }

    Ok(quote)
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args()
        .nth(1)
        .ok_or("请提供行情文件：cargo run -- data/quotes.csv")?;

    let file = File::open(path)?;
    let mut lines = BufReader::new(file).lines();

    let header = lines.next().ok_or("行情文件为空")??;

    if header.trim() != HEADER {
        return Err(format!("第 1 行：表头必须为 {HEADER}").into());
    }

    let mut previous_ts: Option<u64> = None;
    let mut count = 0_u64;

    for (index, line) in lines.enumerate() {
        let line_number = index + 2;
        let text = line.map_err(|e| format!("第 {line_number} 行读取失败：{e}"))?;

        let quote = parse_quote(&text)
            .map_err(|e| format!("第 {line_number} 行：{e}"))?;

        if let Some(previous) = previous_ts {
            if quote.ts_ns < previous {
                return Err(format!(
                    "第 {line_number} 行：时间倒退，上一条为 {previous}，当前为 {}",
                    quote.ts_ns
                )
                .into());
            }
        }

        previous_ts = Some(quote.ts_ns);
        count += 1;

        println!("{quote:?}");
    }

    if count == 0 {
        return Err("文件只有表头，没有行情".into());
    }

    println!("读取完成，共 {count} 条报价");

    Ok(())
}