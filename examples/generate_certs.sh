#!/bin/sh

openssl req -x509 -new -nodes -subj "/C=US/ST=CA/L=Carlsbad/O=None/OU=None/CN=*" -out default_cert.crt -keyout default_cert.key
openssl req -x509 -new -nodes -subj "/C=US/ST=CA/L=Carlsbad/O=None/OU=None/CN=foo.com" -out foo.com.crt -keyout foo.com.key
