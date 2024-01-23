use arboard::Clipboard;
use accessibility_sys::{
    AXUIElementRef,
    AXUIElementCopyAttributeValue,
    AXUIElementCreateSystemWide, 
    AXUIElementCreateApplication,
    AXIsProcessTrustedWithOptions,
    kAXFocusedUIElementAttribute, 
    kAXSelectedTextAttribute, 
    kAXTrustedCheckOptionPrompt, kAXErrorSuccess,
};
use core_foundation::{
    base::CFRelease,
    dictionary::CFDictionary,
    base::{TCFType, TCFTypeRef},
    string::{CFString, CFStringRef},
};
use icrate::AppKit::{NSWorkspace, NSRunningApplication, NSPasteboard};
use icrate::objc2::msg_send;
use log::{error, info};
use std::error::Error;
use std::mem::MaybeUninit;
use libc::pid_t;


pub fn get_text() -> String {
    match get_selected_text_by_accessibility() {
        Ok(text) => {
            if !text.is_empty() {
                return text;
            } else {
                info!("get_selected_text_by_accessibility is empty");
            }
        }
        Err(err) => {
            error!("get_selected_text_by_accessibility error:{}", err);
        }
    }
    info!("fallback to get_text_by_clipboard");
    match get_text_by_clipboard() {
        Ok(text) => {
            if !text.is_empty() {
                return text;
            } else {
                info!("get_text_by_clipboard is empty");
            }
        }
        Err(err) => {
            error!("get_text_by_clipboard error:{}", err);
        }
    }
    // Return Empty String
    String::new()
}

fn def_attr<'a>(ident: &'a str) -> CFString {
    return CFString::new(ident);
}

fn get_selected_text_by_accessibility() -> Result<String, Box<dyn Error>> {
    unsafe {
        let options: CFDictionary<CFString, CFString> = CFDictionary::from_CFType_pairs(
            &[(CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt), def_attr("YES"))]
        );
        let accessibility_enabled = AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef());
        // println!("accessibility_enabled: {:?}", accessibility_enabled);
        
        let app = NSWorkspace::sharedWorkspace().frontmostApplication()
            .unwrap_or(NSRunningApplication::currentApplication());
        let pid: pid_t = msg_send![&app, processIdentifier];
        // println!("{:?}", pid);
        
        let app_ui = AXUIElementCreateApplication(pid);
        let sys_ref = AXUIElementCreateSystemWide();
        // println!("sys_ref: {:?}, app_ui: {:?}", sys_ref, app_ui);
        let mut focused_el  = MaybeUninit::uninit();
        let error1 = AXUIElementCopyAttributeValue(
            app_ui,
            def_attr(kAXFocusedUIElementAttribute).as_concrete_TypeRef(),
            focused_el.as_mut_ptr(),
        );
        
        if error1 != kAXErrorSuccess {
            return Err(format!("Error get focused ui element for {:?}, {:?}", app_ui, error1).into());
        }

        let focused_el_ref = AXUIElementRef::from_void_ptr(focused_el.assume_init());
        let mut selected_text = MaybeUninit::uninit();
        let error2 = AXUIElementCopyAttributeValue(
            focused_el_ref,
            def_attr(kAXSelectedTextAttribute).as_concrete_TypeRef(),
            selected_text.as_mut_ptr(),
        );

        CFRelease(focused_el.assume_init());
        if error2 != kAXErrorSuccess {
            return Err(format!("Error get selected text for {:?}, {:?}", focused_el.assume_init(), error2).into());
        }

        let text = CFStringRef::from_void_ptr(selected_text.assume_init());
        let cf_text = CFString::wrap_under_create_rule(text);
        let str = cf_text.to_string().clone();

        // CFRelease(selected_text.assume_init());
        
        Ok(str)
    //     // !!!: This frame is left-top position
    //     // let selected_text_frame = self.get_selected_text_frame(focused_el_ref);
    }
}

// impl Monitor {
//     fn get_selected_text(&mut self) {
//         self.record_select_text_info();
//         self.is_text_editable = false;
//         self.get_selected_text_by_accessibility();
//     }

//     fn record_select_text_info(&mut self) {
//         unsafe {
//             self.endpoint = NSEvent::mouseLocation();
//             let workspace = NSWorkspace::sharedWorkspace();
//             self.app = workspace.frontmostApplication()
//         }
//     }

//     fn def_attr(ident: &str) -> CFStringRef {
//         return CFString::from_static_string(ident).as_concrete_TypeRef();
//     }

//     // fn get_selected_text_frame(&self, el_ref: AXUIElementRef) -> CGRect {
//     //     unsafe {
//     //         // Ref: https://macdevelopers.wordpress.com/2014/02/05/how-to-get-selected-text-and-its-coordinates-from-any-system-wide-application-using-accessibility-api/

//     //         let selection_range: &mut CFTypeRef;
//     //         let error2 = AXUIElementCopyAttributeValue(
//     //             el_ref,
//     //             Self::def_attr(kAXSelectedTextRangeAttribute),
//     //             selection_range,
//     //         );

//     //         let bounds: &mut CFTypeRef;
//     //         let error3 = AXUIElementCopyParameterizedAttributeValue(
//     //             el_ref,
//     //             Self::def_attr(kAXBoundsForRangeParameterizedAttribute),
//     //             selection_range.as_void_ptr(),
//     //             bounds,
//     //         );

//     //         let selection_frame: &mut c_void;
//     //         let selection_range_ref = AXValueRef::from_void_ptr(selection_range);
//     //         let error4 = AXValueGetValue(selection_range_ref, kAXValueCGRectType, selection_frame);

//     //         CFRelease(selection_frame);
//     //         CFRelease(bounds);
//     //         CFRelease(selection_range);
//     //     }

//     //     return selection_frame;
//     // }
// }

// Available for almost all applications
fn get_text_by_clipboard() -> Result<String, Box<dyn Error>> {
    // Read Old Clipboard
    let old_clipboard = (Clipboard::new()?.get_text(), Clipboard::new()?.get_image());

    if copy() {
        // Read New Clipboard
        let new_text = Clipboard::new()?.get_text();

        // Create Write Clipboard
        let mut write_clipboard = Clipboard::new()?;

        match old_clipboard {
            (Ok(text), _) => {
                // Old Clipboard is Text
                write_clipboard.set_text(text)?;
                if let Ok(new) = new_text {
                    Ok(new.trim().to_string())
                } else {
                    Err("New clipboard is not Text".into())
                }
            }
            (_, Ok(image)) => {
                // Old Clipboard is Image
                write_clipboard.set_image(image)?;
                if let Ok(new) = new_text {
                    Ok(new.trim().to_string())
                } else {
                    Err("New clipboard is not Text".into())
                }
            }
            _ => {
                // Old Clipboard is Empty
                write_clipboard.clear()?;
                if let Ok(new) = new_text {
                    Ok(new.trim().to_string())
                } else {
                    Err("New clipboard is not Text".into())
                }
            }
        }
    } else {
        Err("Copy Failed".into())
    }
}

fn copy() -> bool {
    use enigo::*;
    let num_before = unsafe {
        let pb = NSPasteboard::generalPasteboard();
        pb.changeCount()
    };

    let mut enigo = Enigo::new();
    enigo.key_up(Key::Control);
    enigo.key_up(Key::Alt);
    enigo.key_up(Key::Shift);
    enigo.key_up(Key::Space);
    enigo.key_up(Key::Meta);
    enigo.key_up(Key::Tab);
    enigo.key_up(Key::Escape);
    enigo.key_up(Key::CapsLock);
    // enigo.key_up(Key::C);
    enigo.key_sequence_parse("{+META}c{-META}");
    std::thread::sleep(std::time::Duration::from_millis(100));
    let num_after= unsafe {
        let pb = NSPasteboard::generalPasteboard();
        pb.changeCount()
    };
    num_after != num_before
}

fn get_text_by_clipboard_script() -> Result<String, Box<dyn Error>> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(APPLE_SCRIPT)
        .output()?;
    // check exit code
    if output.status.success() {
        // get output content
        let content = String::from_utf8(output.stdout)?;
        Ok(content)
    } else {
        Err(format!("{output:?}").into())
    }
}

const APPLE_SCRIPT: &str = r#"
use AppleScript version "2.4"
use scripting additions
use framework "Foundation"
use framework "AppKit"

tell application "System Events"
    set frontmostProcess to first process whose frontmost is true
    set appName to name of frontmostProcess
end tell

-- Back up clipboard contents:
set savedClipboard to the clipboard

set thePasteboard to current application's NSPasteboard's generalPasteboard()
set theCount to thePasteboard's changeCount()

-- Copy selected text to clipboard:
tell application "System Events" to keystroke "c" using {command down}
delay 0.1 -- Without this, the clipboard may have stale data.

if thePasteboard's changeCount() is theCount then
    return ""
end if

set theSelectedText to the clipboard

set the clipboard to savedClipboard

theSelectedText
"#;
