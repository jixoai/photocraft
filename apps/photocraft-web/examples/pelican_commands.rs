use serde_json::{Value, json};

const SCALE: f64 = 2.0;
const INK: &str = "#294c50";

fn number(tokens: &mut std::str::SplitWhitespace<'_>) -> Result<f64, String> {
    tokens.next().ok_or("Missing path coordinate")?.parse::<f64>().map(|value| value * SCALE).map_err(|error| error.to_string())
}

fn point(tokens: &mut std::str::SplitWhitespace<'_>) -> Result<Value, String> {
    Ok(json!([number(tokens)?, number(tokens)?]))
}

fn path(source: &str) -> Result<Value, String> {
    let mut tokens = source.split_whitespace();
    let mut knots = Vec::<Value>::new();
    let mut closed = false;
    while let Some(command) = tokens.next() {
        match command {
            "M" | "L" => {
                let anchor = point(&mut tokens)?;
                knots.push(json!({"anchor":anchor,"in":anchor,"out":anchor}));
            }
            "C" => {
                let outgoing = point(&mut tokens)?;
                let incoming = point(&mut tokens)?;
                let anchor = point(&mut tokens)?;
                let previous = knots.last_mut().ok_or("Curve before first point")?;
                previous["out"] = outgoing;
                knots.push(json!({"anchor":anchor,"in":incoming,"out":anchor}));
            }
            "Z" => closed = true,
            other => return Err(format!("Unknown path instruction {other}")),
        }
    }
    Ok(json!({"subpaths":[{"closed":closed,"knots":knots}]}))
}

fn command(plan: &mut Vec<Value>, name: &str, params: Value) {
    plan.push(json!({"command":name,"params":params}));
}

fn layer(plan: &mut Vec<Value>, name: &str) {
    command(plan, "layer.new.layer", json!({"name":name}));
}

fn draw(plan: &mut Vec<Value>, source: &str, fill: Option<&str>, stroke: Option<(&str, f64)>) -> Result<(), String> {
    command(plan, "path.set", json!({"path":path(source)?}));
    if let Some(color) = fill {
        command(plan, "path.fill", json!({"color":color}));
    }
    if let Some((color, width)) = stroke {
        command(plan, "path.stroke", json!({"color":color,"size":width*SCALE,"hardness":1,"tool":"brush"}));
    }
    Ok(())
}

fn line(plan: &mut Vec<Value>, source: &str, color: &str, width: f64) -> Result<(), String> {
    draw(plan, source, None, Some((color, width)))
}

fn ellipse(plan: &mut Vec<Value>, center: [f64; 2], radius: [f64; 2], fill: Option<&str>, stroke: Option<(&str, f64)>) -> Result<(), String> {
    let [center_x, center_y] = center;
    let [radius_x, radius_y] = radius;
    let handle_x = radius_x * 0.55228475;
    let handle_y = radius_y * 0.55228475;
    let source = format!(
        "M {} {} C {} {} {} {} {} {} C {} {} {} {} {} {} C {} {} {} {} {} {} C {} {} {} {} {} {} Z",
        center_x + radius_x,
        center_y,
        center_x + radius_x,
        center_y + handle_y,
        center_x + handle_x,
        center_y + radius_y,
        center_x,
        center_y + radius_y,
        center_x - handle_x,
        center_y + radius_y,
        center_x - radius_x,
        center_y + handle_y,
        center_x - radius_x,
        center_y,
        center_x - radius_x,
        center_y - handle_y,
        center_x - handle_x,
        center_y - radius_y,
        center_x,
        center_y - radius_y,
        center_x + handle_x,
        center_y - radius_y,
        center_x + radius_x,
        center_y - handle_y,
        center_x + radius_x,
        center_y,
    );
    draw(plan, &source, fill, stroke)
}

fn scene() -> Result<Vec<Value>, String> {
    let mut plan = Vec::new();
    command(&mut plan, "file.new", json!({"width":1800,"height":1200,"depth":8,"name":"pelican-cycling"}));
    layer(&mut plan, "01 · 海风与远景");
    command(&mut plan, "paint.gradient", json!({"from":[0,0],"to":[0,1200],"colors":["#e9eee0","#f5f2e3"],"dither":true}));
    ellipse(&mut plan, [194.0, 147.0], [58.0, 58.0], Some("#f2c381"), None)?;
    ellipse(&mut plan, [194.0, 147.0], [52.0, 52.0], Some("#f3c88b"), None)?;
    draw(&mut plan, "M 38 181 C 59 174 62 182 79 175 C 90 149 114 157 122 174 C 142 168 151 173 155 181 Z", Some("#fbfbf0"), None)?;
    draw(&mut plan, "M 653 106 C 665 95 678 100 688 99 C 700 79 721 87 726 99 C 744 94 757 99 766 109 L 653 109 Z", Some("#fbfbef"), None)?;
    line(&mut plan, "M 659 148 C 662 144 668 144 672 148 C 676 144 681 145 684 148", "#8fb4ad", 1.7)?;
    line(&mut plan, "M 721 179 C 724 174 730 175 735 179 C 739 174 745 174 749 178", "#8fb4ad", 1.7)?;
    draw(
        &mut plan,
        "M 0 344 C 52 325 83 324 136 341 C 165 333 207 314 242 332 C 277 350 314 327 351 339 C 400 332 451 340 482 350 L 900 368 L 900 469 L 0 471 Z",
        Some("#adc9bf"),
        None,
    )?;
    draw(
        &mut plan,
        "M 0 357 C 62 334 103 351 144 354 C 195 355 223 331 257 349 C 288 362 341 348 374 354 L 900 362 L 900 467 L 0 470 Z",
        Some("#c6dacf"),
        None,
    )?;
    draw(&mut plan, "M 0 369 C 162 363 249 361 345 367 C 479 377 605 355 900 368 L 900 477 L 0 477 Z", Some("#dae5d6"), None)?;
    for source in [
        "M 46 389 L 105 389",
        "M 23 419 L 76 419",
        "M 58 446 L 123 446",
        "M 694 388 L 724 388",
        "M 678 421 L 740 421",
        "M 760 405 L 804 405",
        "M 591 453 L 668 453",
    ] {
        line(&mut plan, source, "#f7f7e9", 2.2)?;
    }

    layer(&mut plan, "02 · 沙路与接地阴影");
    draw(&mut plan, "M 0 438 C 147 416 265 436 413 438 C 581 445 737 431 900 438 L 900 600 L 0 600 Z", Some("#f5edd4"), None)?;
    draw(&mut plan, "M 0 465 C 159 443 332 449 486 465 C 668 480 778 458 900 465 L 900 600 L 0 600 Z", Some("#f0e1bd"), None)?;
    draw(&mut plan, "M 0 562 C 186 531 361 554 551 550 C 703 550 794 556 900 565 L 900 600 L 0 600 Z", Some("#efddb5"), None)?;
    line(&mut plan, "M 0 557 C 208 525 290 554 474 551 C 643 547 771 551 900 563", "#e7d4aa", 1.3)?;
    ellipse(&mut plan, [462.0, 527.0], [263.0, 14.0], Some("#dbd1ab"), None)?;
    ellipse(&mut plan, [462.0, 529.0], [192.0, 7.0], Some("#d1cba7"), None)?;

    layer(&mut plan, "03 · 车轮、辐条与挡泥板");
    for center_x in [310.0, 615.0] {
        ellipse(&mut plan, [center_x, 434.0], [97.0, 97.0], Some("#efe9ce"), Some((INK, 4.5)))?;
        ellipse(&mut plan, [center_x, 434.0], [90.0, 90.0], None, Some(("#f9f3d9", 3.0)))?;
        ellipse(&mut plan, [center_x, 434.0], [87.0, 87.0], None, Some(("#668f8a", 1.0)))?;
        for spoke in 0..24 {
            let angle = f64::from(spoke) * std::f64::consts::TAU / 24.0;
            let end_x = center_x + angle.cos() * 87.0;
            let end_y = 434.0 + angle.sin() * 87.0;
            line(&mut plan, &format!("M {center_x} 434 L {end_x} {end_y}"), "#829c8f", 0.85)?;
        }
        ellipse(&mut plan, [center_x, 434.0], [9.0, 9.0], Some("#a7c7b5"), Some((INK, 1.6)))?;
    }
    line(&mut plan, "M 214 410 C 224 349 266 323 310 334 C 355 338 383 367 398 398", "#579f9a", 7.0)?;
    line(&mut plan, "M 524 403 C 541 353 574 330 615 334 C 665 337 700 368 709 414", "#579f9a", 7.0)?;
    line(&mut plan, "M 214 410 C 224 349 266 323 310 334 C 355 338 383 367 398 398", "#a0c8b2", 2.0)?;
    line(&mut plan, "M 524 403 C 541 353 574 330 615 334 C 665 337 700 368 709 414", "#a0c8b2", 2.0)?;

    layer(&mut plan, "04 · 蹬踏的腿与脚");
    line(&mut plan, "M 410 324 C 405 348 389 371 394 397 L 411 457", "#c58a3b", 10.0)?;
    line(&mut plan, "M 410 324 C 405 348 389 371 394 397 L 411 454", "#f4c76e", 6.0)?;
    draw(&mut plan, "M 405 454 L 426 457 L 444 465 L 411 467 L 403 462 Z", Some("#edae4f"), Some(("#bc8136", 1.2)))?;

    layer(&mut plan, "05 · 薄荷绿自行车");
    for (source, width) in [
        ("M 310 434 L 407 330 L 453 434 Z", 7.0),
        ("M 407 330 L 555 330 L 453 434", 7.5),
        ("M 453 434 L 589 370", 7.0),
        ("M 555 309 L 615 434", 9.0),
        ("M 559 318 L 603 423", 3.0),
    ] {
        line(&mut plan, source, "#419b96", width)?;
    }
    for source in ["M 313 430 L 406 334", "M 411 334 L 548 334", "M 456 430 L 586 371", "M 561 317 L 610 429"] {
        line(&mut plan, source, "#a0cfb7", 1.9)?;
    }
    draw(&mut plan, "M 384 314 C 405 308 438 310 447 316 L 444 322 L 390 322 Z", Some(INK), None)?;
    line(&mut plan, "M 413 321 L 421 346", INK, 3.3)?;
    line(&mut plan, "M 555 313 L 558 288 C 562 277 579 283 586 287", INK, 6.0)?;
    line(&mut plan, "M 557 313 L 560 290 C 564 282 576 285 583 288", "#abd0b9", 2.2)?;
    line(&mut plan, "M 579 287 L 590 291", "#9b673d", 6.5)?;
    line(&mut plan, "M 589 293 C 616 304 602 346 585 365", INK, 1.2)?;
    ellipse(&mut plan, [450.0, 434.0], [20.0, 20.0], Some("#9ebeb0"), Some((INK, 3.4)))?;
    ellipse(&mut plan, [450.0, 434.0], [12.0, 12.0], Some("#dce0bd"), Some(("#608e85", 1.8)))?;
    line(&mut plan, "M 310 440 L 437 448 C 456 448 464 436 465 431", INK, 2.2)?;
    line(&mut plan, "M 310 428 L 431 420", INK, 2.2)?;
    line(&mut plan, "M 450 434 L 420 465", INK, 4.5)?;
    line(&mut plan, "M 413 465 L 437 465", INK, 4.0)?;
    ellipse(&mut plan, [310.0, 434.0], [5.0, 5.0], Some("#efedcd"), Some((INK, 2.2)))?;
    ellipse(&mut plan, [615.0, 434.0], [5.0, 5.0], Some("#efedcd"), Some((INK, 2.2)))?;

    layer(&mut plan, "06 · 骑车的鹈鹕");
    draw(&mut plan, "M 532 134 C 515 129 516 120 514 113 L 528 123 L 533 105 L 542 124 L 555 114 L 553 132 Z", Some("#fcfcf0"), Some((INK, 1.7)))?;
    draw(
        &mut plan,
        "M 499 276 C 501 252 515 234 526 209 C 537 186 527 174 528 150 C 528 129 546 120 564 124 C 587 125 600 144 590 166 C 572 176 550 189 552 215 C 551 242 566 269 550 291 L 503 306 Z",
        Some("#f9faee"),
        Some((INK, 2.0)),
    )?;
    draw(
        &mut plan,
        "M 538 170 C 544 178 538 201 529 216 C 521 233 516 252 524 274 C 509 265 510 246 517 228 C 528 204 533 188 538 170 Z",
        Some("#e1e5d8"),
        None,
    )?;
    draw(&mut plan, "M 546 189 C 534 217 536 245 547 275 C 536 265 529 240 534 219 Z", Some("#ffffff"), None)?;
    draw(
        &mut plan,
        "M 278 265 C 290 273 313 265 329 259 C 318 245 327 241 349 249 C 370 236 389 238 412 251 C 437 265 454 276 478 271 C 497 275 513 288 537 289 C 526 320 499 337 453 338 C 395 339 337 318 278 265 Z",
        Some("#fbfbee"),
        Some((INK, 2.0)),
    )?;
    draw(
        &mut plan,
        "M 288 269 C 322 290 353 302 389 315 C 370 299 356 283 351 266 C 377 248 413 266 449 284 C 480 300 488 302 510 300 C 469 334 383 326 341 301 Z",
        Some("#d6e1d5"),
        None,
    )?;
    draw(&mut plan, "M 311 274 C 336 281 342 277 351 272 C 345 288 352 297 376 309 C 350 302 329 291 311 274 Z", Some("#fffff5"), None)?;
    draw(&mut plan, "M 357 267 C 376 251 407 262 439 281 C 462 294 481 303 506 307 C 473 335 393 321 357 267 Z", Some("#c7d8cc"), Some(("#99b8ae", 1.1)))?;
    draw(&mut plan, "M 367 267 C 394 257 417 270 443 286 C 459 297 478 303 491 306 C 457 322 400 303 367 267 Z", Some("#f6f8e9"), None)?;
    for source in [
        "M 374 275 C 395 291 413 299 431 304",
        "M 389 275 C 407 289 426 299 447 306",
        "M 405 276 C 425 290 442 299 463 304",
        "M 423 282 C 441 292 456 298 477 301",
    ] {
        line(&mut plan, source, "#a0bfb4", 1.4)?;
    }
    draw(
        &mut plan,
        "M 459 268 C 477 263 493 280 516 287 C 540 294 558 290 576 297 L 588 301 L 581 308 L 575 309 C 586 313 579 316 572 312 L 531 307 C 500 304 478 299 459 268 Z",
        Some("#f9fbee"),
        Some((INK, 1.7)),
    )?;
    line(&mut plan, "M 476 281 C 508 302 544 300 573 305", "#d5e0d4", 2.5)?;
    line(&mut plan, "M 486 326 C 491 344 498 360 502 373 C 509 387 494 393 481 400", "#c78633", 9.0)?;
    line(&mut plan, "M 486 326 C 491 344 498 360 502 373 C 509 387 494 393 481 400", "#f6c76c", 5.8)?;
    draw(&mut plan, "M 481 394 C 488 398 494 400 505 404 L 472 405 L 469 402 Z", Some("#efb65b"), Some(("#b17b32", 1.2)))?;
    ellipse(&mut plan, [565.0, 151.0], [7.1, 8.6], Some("#e7b455"), None)?;
    ellipse(&mut plan, [565.4, 151.0], [3.8, 5.8], Some(INK), None)?;
    ellipse(&mut plan, [566.5, 149.1], [1.4, 1.7], Some("#fff8db"), None)?;
    line(&mut plan, "M 557 141 C 562 136 569 137 574 141", INK, 1.6)?;
    draw(&mut plan, "M 578 165 C 634 166 682 177 747 191 C 712 209 669 230 630 237 C 602 245 587 223 578 165 Z", Some("#e49343"), Some(("#ae7336", 1.8)))?;
    draw(&mut plan, "M 584 174 C 634 183 687 191 736 191 C 709 206 664 226 635 230 C 608 236 594 217 584 174 Z", Some("#f4bb61"), None)?;
    draw(&mut plan, "M 578 163 C 634 164 687 177 749 190 C 695 195 637 185 580 178 Z", Some("#f7c973"), Some(("#a96f32", 1.5)))?;
    line(&mut plan, "M 590 176 C 648 183 695 191 741 190", "#b88138", 1.4)?;
    line(&mut plan, "M 602 191 C 613 225 643 226 675 215", "#ffe1a0", 3.0)?;
    line(&mut plan, "M 592 171 L 601 172", "#8d6430", 1.4)?;

    layer(&mut plan, "07 · 随海风飘动的红领巾");
    draw(&mut plan, "M 510 217 C 491 218 470 210 453 194 L 460 210 L 437 212 C 452 224 485 233 506 224 Z", Some("#c46f50"), Some(("#a75f46", 1.3)))?;
    draw(&mut plan, "M 502 216 C 520 223 534 224 548 222 L 547 232 C 528 233 512 228 498 224 Z", Some("#d88158"), Some(("#ab6047", 1.2)))?;
    ellipse(&mut plan, [509.0, 218.0], [6.0, 8.0], Some("#d58a60"), Some(("#ae6449", 1.1)))?;
    line(&mut plan, "M 445 213 C 470 219 486 224 503 221", "#e59d70", 1.5)?;

    layer(&mut plan, "08 · 海草、花与微风");
    draw(&mut plan, "M 754 357 L 814 357 L 806 367 L 765 367 Z", Some("#7fa9a1"), None)?;
    draw(&mut plan, "M 785 297 L 785 356 L 756 356 Z", Some("#fffff2"), None)?;
    draw(&mut plan, "M 789 312 L 809 354 L 789 354 Z", Some("#f5f6e5"), None)?;
    line(&mut plan, "M 785 296 L 785 358", "#a4c0b2", 1.3)?;
    for source in ["M 193 283 C 213 280 228 283 239 284", "M 214 302 L 246 302", "M 238 319 L 260 319"] {
        line(&mut plan, source, "#a3bfb0", 1.8)?;
    }
    line(&mut plan, "M 101 538 C 105 506 103 483 92 461", "#749d7b", 2.2)?;
    draw(&mut plan, "M 97 497 C 79 497 72 485 72 470 C 88 476 96 484 97 497 Z", Some("#97b28a"), None)?;
    draw(&mut plan, "M 96 476 C 85 465 83 454 86 450 C 102 455 105 468 96 476 Z", Some("#a4b888"), None)?;
    draw(&mut plan, "M 102 518 C 109 497 119 488 130 487 C 127 505 116 513 102 518 Z", Some("#92ad83"), None)?;
    line(&mut plan, "M 101 529 C 92 513 84 507 76 504", "#799f7d", 2.0)?;
    line(&mut plan, "M 130 545 L 142 507", "#90a773", 1.9)?;
    line(&mut plan, "M 861 548 L 868 510", "#7f9f73", 1.9)?;
    for [center_x, center_y] in [[140.0, 508.0], [870.0, 510.0]] {
        for petal in 0..5 {
            let angle = f64::from(petal) * std::f64::consts::TAU / 5.0;
            ellipse(&mut plan, [center_x + angle.cos() * 5.1, center_y + angle.sin() * 5.1], [4.6, 4.6], Some("#d99863"), None)?;
        }
        ellipse(&mut plan, [center_x, center_y], [3.0, 3.0], Some("#f1c77e"), None)?;
    }
    draw(&mut plan, "M 812 548 C 790 538 779 522 777 509 C 793 515 806 529 812 548 Z", Some("#97b38c"), None)?;
    draw(&mut plan, "M 812 548 C 799 517 798 492 806 474 C 816 491 815 526 812 548 Z", Some("#89aa87"), None)?;
    draw(&mut plan, "M 812 548 C 814 515 832 498 850 491 C 846 515 832 538 812 548 Z", Some("#8bac87"), None)?;
    line(&mut plan, "M 811 541 L 806 484", "#becbad", 1.1)?;
    line(&mut plan, "M 818 538 L 844 499", "#c1ccad", 1.0)?;

    layer(&mut plan, "09 · 轻盈纸感");
    for index in 0..72 {
        let center_x = f64::from((index * 137 + 43) % 894) + 3.0;
        let center_y = 552.0 + f64::from((index * 19 + 7) % 43);
        ellipse(&mut plan, [center_x, center_y], [0.75, 0.65], Some("#decea8"), None)?;
    }
    command(&mut plan, "path.delete", json!({}));
    Ok(plan)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    serde_json::to_writer_pretty(std::io::stdout(), &scene().map_err(std::io::Error::other)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_uses_only_discoverable_commands_and_nine_editable_layers() {
        let plan = scene().unwrap();
        assert_eq!(plan.iter().filter(|item| item["command"] == "layer.new.layer").count(), 9);
        for item in plan {
            assert!(photocraft_engine::commands::find(item["command"].as_str().unwrap()).is_some());
        }
        assert!(path("C 1 2 3 4 5 6").is_err());
        assert!(path("M 1").is_err());
    }
}
