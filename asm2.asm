.globl  rslox::compiler::Parser::binary
        .p2align        4
rslox::compiler::Parser::binary:
                // C:\Coding\Projects\learning\loxlang\rslox\src\compiler.rs : 197
                pub fn binary(&mut self) {
.seh_proc _ZN5rslox8compiler6Parser6binary17h06bce2c0dc711085E
        push r14
        .seh_pushreg r14
        push rsi
        .seh_pushreg rsi
        push rdi
        .seh_pushreg rdi
        push rbx
        .seh_pushreg rbx
        sub rsp, 40
        .seh_stackalloc 40
        .seh_endprologue
        mov rsi, rcx
                // C:\Coding\Projects\learning\loxlang\rslox\src\compiler.rs : 198
                let kind = self.prev.kind;
        movzx ecx, byte ptr [rcx + 92]
                // C:\Coding\Projects\learning\loxlang\rslox\src\scanner.rs : 60
                match self {
        add cl, -8
        cmp cl, 4
        jae .LBB39_10
        mov edi, dword ptr [rsi + 88]
        shl cl, 3
        mov edx, 134743815
        shr edx, cl
        mov ebx, 101123077
        shr ebx, cl
                // C:\Coding\Projects\learning\loxlang\rslox\src\compiler.rs : 201
                self.parse_precedence(kind.precedence().incr());
        mov rcx, rsi
        call rslox::compiler::Parser::parse_precedence
                // C:\Coding\Projects\learning\loxlang\rslox\src\compiler.rs : 211
                self.chunk.push_opcode(code, line);
        mov rsi, qword ptr [rsi + 40]
                // C:\Users\ant_b\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib/rustlib/src/rust\library\alloc\src\vec\mod.rs : 2444
                let len = self.len;
        mov r14, qword ptr [rsi + 16]
                // C:\Users\ant_b\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib/rustlib/src/rust\library\alloc\src\vec\mod.rs : 2447
                if len == self.buf.capacity() {
        cmp r14, qword ptr [rsi]
        jne .LBB39_3
                // C:\Users\ant_b\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib/rustlib/src/rust\library\alloc\src\vec\mod.rs : 2448
                self.buf.grow_one();
        lea rdx, [rip + .Lanon.1a0a40a8568da7ccde7c5fec1570dbc1.56]
        mov rcx, rsi
        call alloc::raw_vec::RawVec<T,A>::grow_one
.LBB39_3:
                // C:\Users\ant_b\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib/rustlib/src/rust\library\alloc\src\raw_vec\mod.rs : 512
                self.ptr.cast().as_non_null_ptr()
        mov rax, qword ptr [rsi + 8]
                // C:\Users\ant_b\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib/rustlib/src/rust\library\core\src\ptr\mod.rs : 1655
                intrinsics::write_via_move(dst, src)
        mov byte ptr [rax + r14], bl
                // C:\Users\ant_b\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib/rustlib/src/rust\library\alloc\src\vec\mod.rs : 2453
                self.len = len + 1;
        inc r14
        mov qword ptr [rsi + 16], r14
                // C:\Users\ant_b\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib/rustlib/src/rust\library\core\src\slice\mod.rs : 304
                if let [.., last] = self { Some(last) } else { None }
        mov rax, qword ptr [rsi + 56]
                // C:\Users\ant_b\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib/rustlib/src/rust\library\alloc\src\vec\mod.rs : 1632
                unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len) }
        mov rbx, qword ptr [rsi + 64]
                // C:\Users\ant_b\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib/rustlib/src/rust\library\core\src\slice\mod.rs : 304
                if let [.., last] = self { Some(last) } else { None }
        test rbx, rbx
        sete r8b
        lea rcx, [rax + 8*rbx]
                // C:\Coding\Projects\learning\loxlang\rslox\src\chunk.rs : 113
                match self.lines.last_mut() {
        mov rdx, rcx
        add rdx, -8
        sete r9b
        or r9b, r8b
        jne .LBB39_5
                // C:\Coding\Projects\learning\loxlang\rslox\src\chunk.rs : 114
                Some(l) if l.line == line => l.len += 1,
        cmp dword ptr [rdx], edi
        jne .LBB39_5
        inc dword ptr [rcx - 4]
        jmp .LBB39_9
.LBB39_5:
        lea rcx, [rsi + 48]
                // C:\Users\ant_b\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib/rustlib/src/rust\library\alloc\src\vec\mod.rs : 2447
                if len == self.buf.capacity() {
        cmp rbx, qword ptr [rcx]
        jne .LBB39_7
                // C:\Users\ant_b\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib/rustlib/src/rust\library\alloc\src\vec\mod.rs : 2448
                self.buf.grow_one();
        lea rdx, [rip + .Lanon.1a0a40a8568da7ccde7c5fec1570dbc1.57]
        call alloc::raw_vec::RawVec<T,A>::grow_one
                // C:\Users\ant_b\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib/rustlib/src/rust\library\alloc\src\raw_vec\mod.rs : 512
                self.ptr.cast().as_non_null_ptr()
        mov rax, qword ptr [rsi + 56]
.LBB39_7:
                // C:\Users\ant_b\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib/rustlib/src/rust\library\core\src\ptr\mod.rs : 1655
                intrinsics::write_via_move(dst, src)
        mov dword ptr [rax + 8*rbx], edi
        mov dword ptr [rax + 8*rbx + 4], 1
                // C:\Users\ant_b\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib/rustlib/src/rust\library\alloc\src\vec\mod.rs : 2453
                self.len = len + 1;
        inc rbx
        mov qword ptr [rsi + 64], rbx
.LBB39_9:
                // C:\Coding\Projects\learning\loxlang\rslox\src\compiler.rs : 213
                }
        add rsp, 40
        pop rbx
        pop rdi
        pop rsi
        pop r14
        ret
.LBB39_10:
                // C:\Coding\Projects\learning\loxlang\rslox\src\compiler.rs : 201
                self.parse_precedence(kind.precedence().incr());
        mov rcx, rsi
        mov dl, 1
        call rslox::compiler::Parser::parse_precedence
                // C:\Coding\Projects\learning\loxlang\rslox\src\compiler.rs : 208
                _ => unreachable!()
        lea rcx, [rip + .Lanon.1a0a40a8568da7ccde7c5fec1570dbc1.103]
        lea r8, [rip + .Lanon.1a0a40a8568da7ccde7c5fec1570dbc1.105]
        mov edx, 40
        call core::panicking::panic
        int3