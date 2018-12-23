#!/bin/bash

objdump -C -d -S \
  ${2:-target/x86_64-boringoscore/debug/boringos} | grep $1 --context=16
