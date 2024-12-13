# Sandwich Detector
Code to detect potential sandwich attacks on Solana

## What is a Sandwich Attack?
A sandwich attack is a form of market manipulation and front-running that primarily targets DeFi protocols. It occurs when an attacker "sandwiches" a given transaction by placing two transactions around the victim's transaction. The first transaction is placed before the victim's transaction, typically buying the asset and driving up its price. The second transaction is placed after the victim's transaction, selling the asset at a higher price to profit from the manipulated price difference. 

There are several ways to execute sandwich attacks on Solana, with the most popular method being with Jito bundles. MEV bots, such as the infamous "arsc," have been highly successful in executing sandwich attacks on Solana. Moreover, since Solana lacks a public mempool, certain validators run private mempools that allow them to monitor and exploit pending transactions for sandwich attacks.

## Disclaimer
This tool attempts to identify potential sandwich attacks on Solana. However, due to the complex nature of transactions, there may be false positives or missed detections. Users should perform their own verification and not rely solely on this tool for trading decisions and/or research.

## License
The following code is provided as is under an [MIT license](https://github.com/0xIchigo/sandwich-detector/blob/main/LICENSE)