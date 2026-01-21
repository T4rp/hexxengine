#!/bin/bash

glslc shaders/main.vert -o shaders/base.vert.spv
glslc shaders/main.frag -o shaders/base.frag.spv

glslc shaders/shadow.vert -o shaders/shadow.vert.spv
glslc shaders/shadow.frag -o shaders/shadow.frag.spv

