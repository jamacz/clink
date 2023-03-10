# Clink

Clink is an esoteric functional programming language designed to be extensible, but with a very small number of built-in operations. It is based around an infinite stack containing only `!`s and `?`s, and only has five basic operations:

- `!` - pushes `!` to the stack
- `?` - pushes `?` to the stack
- `:` - pop from the stack, and pattern match
- `@` - read ASCII character and push to stack
- `#` - pop and print ASCII character from top of stack

This is an interpreter for Clink. It's not the best implementation - I could create a compiler or some fancy JIT compilation - but this is more intended to be a proof of concept.

## Installation

Installation is easy. Make sure you have `cargo` installed and run:

```bash
git clone https://github.com/jamacz/clink.git
cd clink
cargo build --release
```

You will find a binary in `./target/release`. Extract to wherever you want.

## Usage

To run clink, navigate to your project folder and run:

``` bash
clink <file>
```

where `<file>` is a `.clink` file.

## Tutorial

### Basics

Create a new file called `main.clink`, for example, in your project folder. Copy the following:

``` txt
_#############?!??!????!!??!?!?!!?!!???!!?!!???!!?!!!!??!??????!!!?!!!?!!?!!!!?!!!??!??!!?!!???!!??!????!????!????!?!?
```

This may look scary, so let's have a look at what it's doing:

`_` is the entry point for the program. Every program with an entry point that you write should have a `_` function. After the function name, you can write the function definition until you reach the end of the file, or a `;` character.

Each function is written in prefix notation, so the first thing to be run in the function is `?!??...!?!?`.
This is simply "Hello world!\n" in ASCII, where every '0' is `?` and every '1' is `!`. This will push the bits for "Hello world!\n", right to left, onto the stack.

`#############` is simply a series of "print" statements. Each `#` pops 8 "bits" from the top of the stack and prints them. We have 13 characters, so 13 `#`s.

This is a hello world program! We're very original.

### Functions

You might be misled into thinking this is hard to read, so we can instead write this as a chain of functions, and format a bit nicer:

``` text
_       ############# H e l l o space w o r l d bang newline;

H       ?!??!???;
e       ?!!??!?!;
l       ?!!?!!??;
o       ?!!?!!!!;
space   ??!?????;
w       ?!!!?!!!;
r       ?!!!??!?;
d       ?!!??!??;
bang    ??!????!;
newline ????!?!?
```

This is still horrible, but it's less horrible. It's slightly better, but not great. Unfortunately, you chose to program in Clink.

### Packages

Why not package them into another file? Create a file called `chars.clink` containing these functions:

``` text
H       ?!??!???;
e       ?!!??!?!;
l       ?!!?!!??;
o       ?!!?!!!!;
space   ??!?????;
w       ?!!!?!!!;
r       ?!!!??!?;
d       ?!!??!??;
bang    ??!????!;
newline ????!?!?
```

We can now rewrite `main.clink` as:

``` text
!chars
_       ############# H e l l o space w o r l d bang newline;
```

The line `!chars` will import the file `chars.clink`.

If we wanted to import a package with multiple files, we could, for example, move `chars.clink` into a folder called `io`, and import `io.chars`:

``` text
!io.chars
_       ############# H e l l o space w o r l d bang newline;
```

We could then import multiple files from the `io` package.

### Pattern matching

What we've done is great, but we only have one way to pop from the stack.

Let's create a `not` function, which takes the bit on top of the stack, inverts it, and pushes it back on the stack:

```txt
not     ?:!
```

Okay, what?

The `:` is always done first. It pops from the stack, and depending on the value:

- if it's a `!`, do the stuff on the left
- if it's a `?`, do the stuff on the right

(But what if the stack is empty? Think again! The stack is infinite (not actually, but that's not important), and initialised with `?`s.)

So what this function does, in English, is:

- pop from stack
    - if it's a `!`, push `?`
    - if it's a `?`, push `!`

Let's write a more complex function, the `or` function:

``` txt
or      !(:):
```

This code first splits the function into `!(:)` and ` `.

So if it sees a `!`, it runs `!(:)` from left to right. It sees `(:)`, enters the brackets, then pops and matches. Only this time, both sides of the expression are blank, as there is nothing on either side of `:` inside the brackets. 

So `(:)` will pop from the stack, and do nothing, no matter what is on top of the stack. Finally, it pushes a `!` back on the stack, to signal one of the two bits was `!`.

If it sees a `?`, again, it does nothing. It doesn't need to pop from the stack, since what is on top of the stack already is our answer.

### Recursion

A language wouldn't be Turing-complete without some form of iteration. Let's make a function that pops a sequence of `!`s from the top of the stack:

```txt
pop_bangs   pop_bangs:?
```

If it pops a `!`, it calls itself. Otherwise, if it pops a `?`, it simply pushes another `?` back on the stack.

## FAQs

### Why?

I initially had the idea to create a language with only Option and Unit types, but I thought that would be too impractical.
The next iteration included Either and Unit types, but I figured I could abstract the concept even more and only included an Either type in the final iteration.
I worked out that you could think of this as effectively a stack of booleans, and... here we are!

### Why 'Clink'?

It sounded funny.

### Is this practical?

No.

### Would you recommend programming in this?

It's a fun experiment to see how much you can add to the language. I'm currently working on a library for Clink to simulate a simple stack machine.

### What were your inspirations?

The inner workings of my mind.