English WordNet Editor (EWE)
============================

English WordNet Editor is an editor for working with Open English WordNet, which
is developed at https://github.com/globalwordnet/english-wordnet. This ensures
that changes are consistent and validates the resulting files.

Installation
------------

Release builds can be obtained from the [release section](https://github.com/jmccrae/ewe/releases). These are executables and can be run directly. 
We recommend saving these to the same folder that contains the Git repository
for Open English WordNet. EWE can be started by executing this file

Usage
-----

EWE is menu-driven, please choose the appropriate option when it has started
you should see something like this:

```

         ,ww                             
   wWWWWWWW_)  Welcome to EWE            
   `WWWWWW'    - English WordNet Editor  
    II  II                               

Loading WordNet
████████████████████████████████████████████████████████████████████████ 73/73

Please choose an option:
1. Add/delete/move entry
2. Add/delete a synset
3. Change a definition
4. Change an example
5. Change a relation
6. Validate
X. Exit EWE
Option> 
```

Building EWE
------------

EWE can be built with Cargo as follows

    cargo build --release
