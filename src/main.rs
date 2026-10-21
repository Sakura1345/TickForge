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

// 策略每次被调用时，只能读取这次交付给它的上下文。
struct StrategyContext<'a> {
    now_ns: u64,
    quote: &'a Quote,
}

// 所有策略都通过这个接口接收报价。
trait Strategy {
    fn on_quote(&mut self, ctx: &StrategyContext<'_>);
}

// 第一个策略：观察报价，打印价差。
struct PrintStrategy;

impl Strategy for PrintStrategy {
    fn on_quote(&mut self, ctx: &StrategyContext<'_>) {
        let quote = ctx.quote;
        let spread = quote.ask_price - quote.bid_price;

        println!(
            "time={} bid={}x{} ask={}x{} spread={}",
            ctx.now_ns,
            quote.bid_price,
            quote.bid_qty,
            quote.ask_price,
            quote.ask_qty,
            spread,
        );
    }
}

// 一次回放的时间、当前市场状态和策略，都由这个对象持有。
struct ReplayEngine<S> {
    now_ns: Option<u64>,
    latest_quote: Option<Quote>,
    strategy: S,
}

impl<S: Strategy> ReplayEngine<S> {
    fn new(strategy: S) -> Self {
        Self {
            now_ns: None,
            latest_quote: None,
            strategy,
        }
    }

    fn on_quote(&mut self, quote: Quote) -> Result<(), String> {
        let now_ns = quote.ts_ns;

        // 先检查顺序。检查失败时，不修改运行状态。
        if let Some(previous) = self.now_ns {
            if now_ns < previous {
                return Err(format!(
                    "时间倒退，上一条为 {previous}，当前为 {now_ns}"
                ));
            }
        }

        // 推进虚拟时间，并替换当前报价。
        self.now_ns = Some(now_ns);
        self.latest_quote = Some(quote);

        // 此处刚刚写入报价，因此一定存在。
        let current_quote = self
            .latest_quote
            .as_ref()
            .expect("当前报价应当已经写入");

        let ctx = StrategyContext {
            now_ns,
            quote: current_quote,
        };

        // 策略看到更新后的市场状态。
        self.strategy.on_quote(&ctx);

        Ok(())
    }
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

    let mut engine = ReplayEngine::new(PrintStrategy);
    let mut count = 0_u64;

    for (index, line) in lines.enumerate() {
        let line_number = index + 2;
        let text = line.map_err(|e| format!("第 {line_number} 行读取失败：{e}"))?;

        let quote = parse_quote(&text)
            .map_err(|e| format!("第 {line_number} 行：{e}"))?;

        engine
            .on_quote(quote)
            .map_err(|e| format!("第 {line_number} 行：{e}"))?;

        count += 1;
    }

    if count == 0 {
        return Err("文件只有表头，没有行情".into());
    }

    println!("回放完成，共处理 {count} 条报价");

    Ok(())
}