#!/usr/bin/env bash

set -eu

mkdir -p root intermediate
touch {root,intermediate}/index
echo 00 | tee {root,intermediate}/serial

export CA=Root
openssl genrsa 4096 | openssl pkcs8 -topk8 -nocrypt -out root/key.pem
openssl req -new -config ca.cnf -key root/key.pem \
        -out root/csr.pem
openssl ca -batch -config ca.cnf -name root_ca -extensions root_ext \
        -selfsign -keyfile root/key.pem \
        -in root/csr.pem -out root/cert.pem

export CA=Intermediate
openssl genrsa 4096 | openssl pkcs8 -topk8 -nocrypt -out intermediate/key.pem
openssl req -new -config ca.cnf -key intermediate/key.pem \
        -out intermediate/csr.pem
openssl ca -batch -config ca.cnf -name root_ca \
        -cert root/cert.pem -keyfile root/key.pem \
        -in intermediate/csr.pem -out intermediate/cert.pem

export CLIENT=localhost
openssl genrsa 4096 | openssl pkcs8 -topk8 -nocrypt -out ${CLIENT}.key.pem
openssl req -new -config client.cnf -key ${CLIENT}.key.pem \
	-out ${CLIENT}.csr.pem
openssl ca -batch -config ca.cnf -name intermediate_ca \
	-cert intermediate/cert.pem -keyfile intermediate/key.pem \
	-in ${CLIENT}.csr.pem -out ${CLIENT}.cert.pem
openssl verify -CAfile root/cert.pem -untrusted intermediate/cert.pem \
	localhost.cert.pem

# Generate PKCS#12, see github.com/sfackler/rust-native-tls#27
openssl pkcs12 -export -nodes -password pass: \
	-in $CLIENT.cert.pem -certfile intermediate/cert.pem \
	-inkey $CLIENT.key.pem > $CLIENT.p12
