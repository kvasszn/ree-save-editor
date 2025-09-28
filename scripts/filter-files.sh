#!/bin/sh
base=$1
filter=$2
shift 2
subs=("$@")
base_real=$(realpath "$base")
base_real=${base_real%/}
for sub in "${subs[@]}"; do
    find "$sub" -type f -name "$filter" | sed "s|^$base_real/||"
done

find "$base_real/natives/STM/GUI" -type f -name "*.gcp*" | sed "s|^$base_real/||"
find "$base_real/natives/STM/GUI" -type f -name "*.tex*" | sed "s|^$base_real/||"
find "$base_real/natives/STM/GameDesign" -type f -name "*.msg*" | sed "s|^$base_real/||"
