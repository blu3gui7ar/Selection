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
use icrate::AppKit::{NSWorkspace, NSRunningApplication};
use icrate::objc2::msg_send;
use log::{error, info};
use std::error::Error;
use std::mem::MaybeUninit;
use libc::pid_t;


pub fn get_text() -> String {
    match get_text_ways() {
        Ok(text) => {
            if !text.is_empty() {
                return text;
            }
        }
        Err(err) => {
            error!("{}", err)
        }
    }
    // Return Empty String
    String::new()
}

fn get_text_ways() -> Result<String, Box<dyn Error>> {
    let text = get_selected_text_by_accessibility()?;
    if text.is_empty() {
        return get_text_by_clipboard();
    } else {
        Ok(text)
    }
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

fn get_text_by_clipboard() -> Result<String, Box<dyn Error>> {
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
