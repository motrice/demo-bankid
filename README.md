# Readme

Tiny demo with BankID integration

`docker build -t demobankid .`

`docker run --init -p 3001:3001 demobankid:latest`


or just 

`docker run --init -p 3001:3001 motrice/demobankid:latest`

Rust code uses async/await so build with beta toolchain until November 7.

- http://localhost:3001/auth
- http://localhost:3001/sign?userVisibleData=demohello