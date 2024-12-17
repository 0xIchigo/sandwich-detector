<p align="center">
  <img src="/public/assets/Super_Shaggy_Sandwich.png" alt="Shaggy holding a sandwich lmao" width="600"/>
</p>

# Shaggy: The Solana Sandwich Detector
Code anyone can run to detect potential sandwich attacks on Solana

## What is a Sandwich Attack?
A sandwich attack is a form of market manipulation and front-running that primarily targets DeFi protocols. It occurs when an attacker "sandwiches" a given transaction by placing two transactions around the victim's transaction. The first transaction is placed before the victim's transaction, typically buying the asset and driving up its price. The second transaction is placed after the victim's transaction, selling the asset at a higher price to profit from the manipulated price difference. 

There are several ways to execute sandwich attacks on Solana, with the most popular method being with Jito bundles. MEV bots, such as the infamous "arsc," have been highly successful in executing sandwich attacks on Solana. Moreover, since Solana lacks a public mempool, certain validators run private mempools that allow them to monitor and exploit pending transactions for sandwich attacks.

## Disclaimer
This tool attempts to identify potential sandwich attacks on Solana pertaining to the target program `vpeNALD89BZ4KxNUFjdLmFXBCwtyqBDQ85ouNoax38b`. In the future, this tool will be expanded to detect sandwich attacks on Solana more generally. Note that due to the complex nature of these transactions, there may be false positives or missed detections. Users should perform their own verification and not rely solely on this tool for trading decisions and/or research.

## License
The following code is provided as is under an [MIT license](https://github.com/0xIchigo/sandwich-detector/blob/main/LICENSE)