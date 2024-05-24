/*
 * Copyright (c) 1996, 2020, Oracle and/or its affiliates. All rights reserved.
 * DO NOT ALTER OR REMOVE COPYRIGHT NOTICES OR THIS FILE HEADER.
 *
 * This code is free software; you can redistribute it and/or modify it
 * under the terms of the GNU General Public License version 2 only, as
 * published by the Free Software Foundation.  Oracle designates this
 * particular file as subject to the "Classpath" exception as provided
 * by Oracle in the LICENSE file that accompanied this code.
 *
 * This code is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
 * FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License
 * version 2 for more details (a copy is included in the LICENSE file that
 * accompanied this code).
 *
 * You should have received a copy of the GNU General Public License version
 * 2 along with this work; if not, write to the Free Software Foundation,
 * Inc., 51 Franklin St, Fifth Floor, Boston, MA 02110-1301 USA.
 *
 * Please contact Oracle, 500 Oracle Parkway, Redwood Shores, CA 94065 USA
 * or visit www.oracle.com if you need additional information or have any
 * questions.
 */

#ifndef _JAVASOFT_JNI_MD_H_
#define _JAVASOFT_JNI_MD_H_

#ifdef _WIN32
  #ifndef JNIEXPORT
    #define JNIEXPORT __declspec(dllexport)
  #endif
  #define JNIIMPORT __declspec(dllimport)
  #define JNICALL __stdcall
  // 'long' is always 32 bit on windows so this matches what jdk expects
  typedef long jint;
  typedef __int64 jlong;
  typedef signed char jbyte;
#else
  #define JNIEXPORT __attribute__ ((visibility ("default")))
  #define JNIIMPORT
  #define JNICALL
  #if defined(__LP64__) && __LP64__ /* for -Wundef */
  typedef int jint;
  #else
  typedef long jint;
  #endif
  typedef long long jlong;
  typedef signed char jbyte;
#endif


#endif /* !_JAVASOFT_JNI_MD_H_ */
