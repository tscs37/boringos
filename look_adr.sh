#!/bin/bash

objdump -C -d target/x86_64-boringoscore/debug/boringos | grep $1 --context=2
