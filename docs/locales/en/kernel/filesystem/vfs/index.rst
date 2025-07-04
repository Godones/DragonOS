.. note:: AI Translation Notice

   This document was automatically translated by `hunyuan-turbos-latest` model, for reference only.

   - Source document: kernel/filesystem/vfs/index.rst

   - Translation time: 2025-06-29 09:58:04

   - Translation model: `hunyuan-turbos-latest`


   Please report issues via `Community Channel <https://github.com/DragonOS-Community/DragonOS/issues>`_

VFS Virtual File System  
====================================  

In DragonOS, VFS acts as an adapter that abstracts the differences between specific file systems, providing a unified file operation interface to the outside.  

VFS is the core of DragonOS's file system, offering a standardized set of file system interfaces that enable DragonOS to support multiple different file systems. The main functions of VFS include:  

- Providing a unified file system interface  
- Offering file system mount and unmount mechanisms (MountFS)  
- Providing file abstraction (File)  
- Providing file system abstraction (FileSystem)  
- Offering IndexNode abstraction  
- Providing file system caching and synchronization mechanisms (not yet implemented)  
- Supporting the mounting of disk devices onto the file system (currently supports EXT4 and vfat types of virtio disks)  

.. toctree::  
   :maxdepth: 1  
   :caption: Table of Contents  

   design  
   api  
   mountable_fs
