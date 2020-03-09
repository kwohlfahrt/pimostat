#!/usr/bin/env bash

set -eu

touch {root,intermediate}/index
echo 00 | tee {root,intermediate}/serial

openssl genrsa 4096 | openssl pkcs8 -topk8 -nocrypt -out root/key.pem
openssl req -new -config root/config.cnf -key root/key.pem \
        -out root/csr.pem
openssl ca -batch -config root/config.cnf -extensions root_ext \
        -selfsign -keyfile root/key.pem \
        -in root/csr.pem -out root/cert.pem

openssl genrsa 4096 | openssl pkcs8 -topk8 -nocrypt -out intermediate/key.pem
openssl req -new -config intermediate/config.cnf -key intermediate/key.pem \
        -out intermediate/csr.pem
openssl ca -batch -config root/config.cnf \
        -cert root/cert.pem -keyfile root/key.pem \
        -in intermediate/csr.pem -out intermediate/cert.pem

export CLIENT
for CLIENT in sensor controller; do
    openssl genrsa 4096 | openssl pkcs8 -topk8 -nocrypt -out ${CLIENT}.key.pem
    openssl req -new -config client.cnf -key ${CLIENT}.key.pem \
	    -out ${CLIENT}.csr.pem
    openssl ca -batch -config intermediate/config.cnf \
	    -cert intermediate/cert.pem -keyfile intermediate/key.pem \
	    -in ${CLIENT}.csr.pem -out ${CLIENT}.cert.pem
done
openssl verify -CAfile root/cert.pem -untrusted intermediate/cert.pem \
	{sensor,controller}.cert.pem
