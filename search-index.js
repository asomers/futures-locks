var searchIndex = JSON.parse('{\
"futures_locks":{"doc":"A library of [`Futures`]-aware locking primitives. These…","i":[[3,"Mutex","futures_locks","A Futures-aware Mutex.",null,null],[3,"MutexFut","","A `Future` representing a pending `Mutex` acquisition.",null,null],[3,"MutexGuard","","An RAII mutex guard, much like `std::sync::MutexGuard`.…",null,null],[3,"MutexWeak","","`MutexWeak` is a non-owning reference to a [`Mutex`].…",null,null],[3,"RwLock","","A Futures-aware RwLock.",null,null],[3,"RwLockReadFut","","A `Future` representing a pending `RwLock` shared…",null,null],[3,"RwLockWriteFut","","A `Future` representing a pending `RwLock` exclusive…",null,null],[3,"RwLockReadGuard","","An RAII guard, much like `std::sync::RwLockReadGuard`. The…",null,null],[3,"RwLockWriteGuard","","An RAII guard, much like `std::sync::RwLockWriteGuard`.…",null,null],[11,"upgrade","","Tries to upgrade the `MutexWeak` to `Mutex`. If the…",0,[[],[["option",4],["mutex",3]]]],[11,"new","","Create a new `Mutex` in the unlocked state.",1,[[],["mutex",3]]],[11,"try_unwrap","","Consumes the `Mutex` and returns the wrapped data. If the…",1,[[],[["result",4],["mutex",3]]]],[11,"downgrade","","Create a [`MutexWeak`] reference to this `Mutex`.",1,[[["mutex",3]],["mutexweak",3]]],[11,"get_mut","","Returns a reference to the underlying data, if there are…",1,[[],["option",4]]],[11,"lock","","Acquires a `Mutex`, blocking the task in the meantime.…",1,[[],["mutexfut",3]]],[11,"try_lock","","Attempts to acquire the lock.",1,[[],[["mutexguard",3],["result",4]]]],[11,"ptr_eq","","Returns true if the two `Mutex` point to the same data…",1,[[["mutex",3]]]],[11,"with","","Acquires a `Mutex` and performs a computation on its…",1,[[]]],[11,"with_local","","Like `with` but for Futures that aren\'t `Send`. Spawns a…",1,[[]]],[11,"new","","Create a new `RwLock` in the unlocked state.",2,[[],["rwlock",3]]],[11,"try_unwrap","","Consumes the `RwLock` and returns the wrapped data. If the…",2,[[],[["rwlock",3],["result",4]]]],[11,"get_mut","","Returns a reference to the underlying data, if there are…",2,[[],["option",4]]],[11,"read","","Acquire the `RwLock` nonexclusively, read-only, blocking…",2,[[],["rwlockreadfut",3]]],[11,"write","","Acquire the `RwLock` exclusively, read-write, blocking the…",2,[[],["rwlockwritefut",3]]],[11,"try_read","","Attempts to acquire the `RwLock` nonexclusively.",2,[[],[["result",4],["rwlockreadguard",3]]]],[11,"try_write","","Attempts to acquire the `RwLock` exclusively.",2,[[],[["result",4],["rwlockwriteguard",3]]]],[11,"with_read","","Acquires a `RwLock` nonexclusively and performs a…",2,[[]]],[11,"with_read_local","","Like `with_read` but for Futures that aren\'t `Send`.…",2,[[]]],[11,"with_write","","Acquires a `RwLock` exclusively and performs a computation…",2,[[]]],[11,"with_write_local","","Like `with_write` but for Futures that aren\'t `Send`.…",2,[[]]],[11,"from","","",1,[[]]],[11,"into","","",1,[[]]],[11,"to_owned","","",1,[[]]],[11,"clone_into","","",1,[[]]],[11,"try_from","","",1,[[],["result",4]]],[11,"try_into","","",1,[[],["result",4]]],[11,"borrow","","",1,[[]]],[11,"borrow_mut","","",1,[[]]],[11,"type_id","","",1,[[],["typeid",3]]],[11,"from","","",3,[[]]],[11,"into","","",3,[[]]],[11,"try_from","","",3,[[],["result",4]]],[11,"try_into","","",3,[[],["result",4]]],[11,"borrow","","",3,[[]]],[11,"borrow_mut","","",3,[[]]],[11,"type_id","","",3,[[],["typeid",3]]],[11,"into_future","","",3,[[]]],[11,"from","","",4,[[]]],[11,"into","","",4,[[]]],[11,"try_from","","",4,[[],["result",4]]],[11,"try_into","","",4,[[],["result",4]]],[11,"borrow","","",4,[[]]],[11,"borrow_mut","","",4,[[]]],[11,"type_id","","",4,[[],["typeid",3]]],[11,"from","","",0,[[]]],[11,"into","","",0,[[]]],[11,"to_owned","","",0,[[]]],[11,"clone_into","","",0,[[]]],[11,"try_from","","",0,[[],["result",4]]],[11,"try_into","","",0,[[],["result",4]]],[11,"borrow","","",0,[[]]],[11,"borrow_mut","","",0,[[]]],[11,"type_id","","",0,[[],["typeid",3]]],[11,"from","","",2,[[]]],[11,"into","","",2,[[]]],[11,"to_owned","","",2,[[]]],[11,"clone_into","","",2,[[]]],[11,"try_from","","",2,[[],["result",4]]],[11,"try_into","","",2,[[],["result",4]]],[11,"borrow","","",2,[[]]],[11,"borrow_mut","","",2,[[]]],[11,"type_id","","",2,[[],["typeid",3]]],[11,"from","","",5,[[]]],[11,"into","","",5,[[]]],[11,"try_from","","",5,[[],["result",4]]],[11,"try_into","","",5,[[],["result",4]]],[11,"borrow","","",5,[[]]],[11,"borrow_mut","","",5,[[]]],[11,"type_id","","",5,[[],["typeid",3]]],[11,"into_future","","",5,[[]]],[11,"from","","",6,[[]]],[11,"into","","",6,[[]]],[11,"try_from","","",6,[[],["result",4]]],[11,"try_into","","",6,[[],["result",4]]],[11,"borrow","","",6,[[]]],[11,"borrow_mut","","",6,[[]]],[11,"type_id","","",6,[[],["typeid",3]]],[11,"into_future","","",6,[[]]],[11,"from","","",7,[[]]],[11,"into","","",7,[[]]],[11,"try_from","","",7,[[],["result",4]]],[11,"try_into","","",7,[[],["result",4]]],[11,"borrow","","",7,[[]]],[11,"borrow_mut","","",7,[[]]],[11,"type_id","","",7,[[],["typeid",3]]],[11,"from","","",8,[[]]],[11,"into","","",8,[[]]],[11,"try_from","","",8,[[],["result",4]]],[11,"try_into","","",8,[[],["result",4]]],[11,"borrow","","",8,[[]]],[11,"borrow_mut","","",8,[[]]],[11,"type_id","","",8,[[],["typeid",3]]],[11,"drop","","",4,[[]]],[11,"drop","","",3,[[]]],[11,"drop","","",7,[[]]],[11,"drop","","",8,[[]]],[11,"drop","","",5,[[]]],[11,"drop","","",6,[[]]],[11,"clone","","",0,[[],["mutexweak",3]]],[11,"clone","","",1,[[],["mutex",3]]],[11,"clone","","",2,[[],["rwlock",3]]],[11,"default","","",1,[[],["mutex",3]]],[11,"default","","",2,[[],["rwlock",3]]],[11,"deref","","",4,[[]]],[11,"deref","","",7,[[]]],[11,"deref","","",8,[[]]],[11,"deref_mut","","",4,[[]]],[11,"deref_mut","","",8,[[]]],[11,"fmt","","",4,[[["formatter",3]],["result",6]]],[11,"fmt","","",0,[[["formatter",3]],["result",6]]],[11,"fmt","","",1,[[["formatter",3]],["result",6]]],[11,"fmt","","",7,[[["formatter",3]],["result",6]]],[11,"fmt","","",8,[[["formatter",3]],["result",6]]],[11,"fmt","","",2,[[["formatter",3]],["result",6]]],[11,"poll","","",3,[[["context",3],["pin",3]],["poll",4]]],[11,"poll","","",5,[[["context",3],["pin",3]],["poll",4]]],[11,"poll","","",6,[[["context",3],["pin",3]],["poll",4]]]],"p":[[3,"MutexWeak"],[3,"Mutex"],[3,"RwLock"],[3,"MutexFut"],[3,"MutexGuard"],[3,"RwLockReadFut"],[3,"RwLockWriteFut"],[3,"RwLockReadGuard"],[3,"RwLockWriteGuard"]]}\
}');
addSearchOptions(searchIndex);initSearch(searchIndex);