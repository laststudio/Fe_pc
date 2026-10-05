## Third-Party Licenses

Our software includes components from third-party projects. Below are the details for these components:


### Flat Assembler (FASM)
Copyright (c) 1999-2013, Tomasz Grysztar.

The full license text can be found in the `licenses` folder or at `third_party_licenses/FASM_LICENSE.txt`.

We use Flat Assembler to assemble many parts of the protection code that is appended to the protected binary.


### XED Library
XED Library is licensed under the Apache License 2.0.

The full license text can be found in the `licenses` folder or at `third_party_licenses/APACHE_LICENSE_2.0.txt`.

We use the XED library to get the details of specific x86/x64 opcodes and also to generate specific opcodes.


### BeaEngine Library
BeaEngine is licensed under the LGPL. You can use your own version of the BeaEngine library by placing the BeaEngine.dll (x86)
in the same folder where you have the protection software installed.

The full license text can be found in the `licenses` folder or at `third_party_licenses/COPYING` and `third_party_licenses/COPYING.LESSER`.

We use the BeaEngine library to show the disassembled x86/x64 instructions in the User Interface and get the size of specific x86/x64 instructions.
