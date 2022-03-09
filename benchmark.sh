#!/bin/env bash

scenes=("example_scenes/monkey.json" "example_scenes/teapot.json")
resolutions=("128x128" "256x512" "900x200")
baselines=("0.150" "1.235" "0.572" "0.571" "4.143" "4.442")
total="0.0"
index=0

for scene in "${scenes[@]}"
do
	for res in "${resolutions[@]}"
	do
		time="$( raytrs -q -s "$scene" -d "$res" "$@" | awk '{ print $4}' )"
		echo "benchmarking \"$scene\" at $res ..."
		score="$( echo "($time/${baselines[$index]}) * 100" | bc -l )"
		echo "$score"
		total="$( echo "$total + $score" | bc -l )"
		index="$( expr $index + 1 )"
	done
done

score="$( echo "($total / $index)" | bc -l )"
echo "score (lower is better): $score"
