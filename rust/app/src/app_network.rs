//!  Send sensor data to a CoAP Server or a Collector Node.  The CoAP payload will be encoded as JSON
//!  for CoAP Server and CBOR for Collector Node.  The sensor data will be transmitted to 
//!  CoAP Server over WiFi via the ESP8266 transceiver, and to Collector Node via nRF24L01 transceiver.
//!  This enables transmission of Sensor Data to a local Sensor Network (via nRF24L01)
//!  and to the internet (via ESP8266).  For sending to Collector Node we use raw temperature (integer) 
//!  instead of computed temperature (floating-point) to make the encoding simpler and faster.
//!  Note that we are using a patched version of apps/my_sensor_app/src/vsscanf.c that
//!  fixes ESP8266 response parsing bugs.  The patched file must be present in that location.
//!  This is the Rust version of `https://github.com/lupyuen/stm32bluepill-mynewt-sensor/blob/rust/apps/my_sensor_app/OLDsrc/send_coap.c`

use cstr_core::CStr;      //  Import string utilities from `cstr_core` library: https://crates.io/crates/cstr_core
use cty::*;               //  Import C types from cty library: https://crates.io/crates/cty
use macros::{out, strn, init_strn} ; //  Import procedural macros
use mynewt::{
    result::*,            //  Import Mynewt result and error types
    kernel::os::{  
        self,             //  Import Mynewt OS functions
        os_task,          //  Import Mynewt OS types
        os_stack_t,
        os_task_func_t,
        os_time_t,
    },
    encoding::{
        coap_context::{   //  Import Mynewt JSON Encoder Context
            self,
            COAP_CONTEXT,
            ToBytesOptionalNull,
        },
        tinycbor,         //  Import Mynewt TinyCBOR API
    },
    libs::{
        mynewt_rust,      //  Import Mynewt Rust Helper API
        sensor_network,   //  Import Mynewt Sensor Network API
        sensor_coap::{    //  Import Mynewt Sensor CoAP API
            self,
            sensor_value,
        },
    },
    coap, d, fill_zero,   //  Import Mynewt macros
    NULL, Out, Ptr, Strn,
};
use crate::base::*;       //  Import `base.rs` for common declarations

///////////////////////////////////////////////////////////////////////////////
//  Testing

static _test_static: Strn = init_strn!("hello");

fn test_safe_wrap() -> MynewtResult<()> {
    let _test_local = init_strn!("hello");

    "-------------------------------------------------------------";
    #[macros::safe_wrap(attr)]
    extern "C" {
        pub fn os_task_init(
            t: *mut os_task,
            name: *const ::cty::c_char,
            func: os_task_func_t,
            arg: *mut ::cty::c_void,
            prio: u8,
            sanity_itvl: os_time_t,
            stack_bottom: *mut os_stack_t,
            stack_size: u16,
        ) -> ::cty::c_int;
    }
    "-------------------------------------------------------------";

    task_init(                      //  Create a new task and start it...
        out!( NETWORK_TASK ),       //  Task object will be saved here
        strn!( "network" ),         //  Name of task
        Some( network_task_func ),  //  Function to execute when task starts
        NULL,  //  Argument to be passed to above function
        10,    //  Task priority: highest is 0, lowest is 255 (main task is 127)
        os::OS_WAIT_FOREVER as u32,   //  Don't do sanity / watchdog checking
        out!( NETWORK_TASK_STACK ),   //  Stack space for the task
        NETWORK_TASK_STACK_SIZE as u16 //  Size of the stack (in 4-byte units)
    )?;                                //  `?` means check for error

    pub fn OLDtask_init(
        t: Out<os_task>,  //  TODO: *mut os_task
        name: &Strn,      //  TODO: *const ::cty::c_char
        func: os_task_func_t,
        arg: Ptr,         //  TODO: *mut ::cty::c_void
        prio: u8,
        sanity_itvl: os_time_t,
        stack_bottom: Out<[os_stack_t]>,  //  TODO: *mut os_stack_t
        stack_size: usize,                //  TODO: u16
    ) -> MynewtResult<()> {               //  TODO: ::cty::c_int;
        extern "C" {
            pub fn os_task_init(
                t: *mut os_task,
                name: *const ::cty::c_char,
                func: os_task_func_t,
                arg: *mut ::cty::c_void,
                prio: u8,
                sanity_itvl: os_time_t,
                stack_bottom: *mut os_stack_t,
                stack_size: u16,
            ) -> ::cty::c_int;
        }
        Strn::validate_bytestr(name.bytestr);  //  TODO
        unsafe {
            let res = os_task_init(
                t,
                name.bytestr.as_ptr() as *const ::cty::c_char,  //  TODO
                func,
                arg,
                prio,
                sanity_itvl,
                stack_bottom.as_ptr() as *mut os_stack_t,  //  TODO
                stack_size as u16       //  TODO
            );
            if res == 0 { Ok(()) }
            else { Err(MynewtError::from(res)) }
        }
    }

        #[doc = " Initialize a task."]
        #[doc = ""]
        #[doc = " This function initializes the task structure pointed to by t,"]
        #[doc = " clearing and setting it's stack pointer, provides sane defaults"]
        #[doc = " and sets the task as ready to run, and inserts it into the operating"]
        #[doc = " system scheduler."]
        #[doc = ""]
        #[doc = " - __`t`__: The task to initialize"]
        #[doc = " - __`name`__: The name of the task to initialize"]
        #[doc = " - __`func`__: The task function to call"]
        #[doc = " - __`arg`__: The argument to pass to this task function"]
        #[doc = " - __`prio`__: The priority at which to run this task"]
        #[doc = " - __`sanity_itvl`__: The time at which this task should check in with the"]
        #[doc = "                    sanity task.  OS_WAIT_FOREVER means never check in"]
        #[doc = "                    here."]
        #[doc = " - __`stack_bottom`__: A pointer to the bottom of a task's stack"]
        #[doc = " - __`stack_size`__: The overall size of the task's stack."]
        #[doc = ""]
        #[doc = " Return: 0 on success, non-zero on failure."]
        fn dummy() {}
    Ok(())
}

/// Run a block of CBOR encoding calls with error checking.
#[cfg(NOTUSED)]
fn test_run() {
    let key_with_opt_null = "";
    let value = 1;
    "-------------------------------------------------------------";
    mynewt_macros::run!({
        //  TODO: First para should be name of current map or array
        let encoder = JSON_CONTEXT.encoder("JSON_CONTEXT", "_map");
        //  d!(> TODO: g_err |= cbor_encode_text_string(&object##_map, #key, strlen(#key)));
        cbor_encode_text_string(
            encoder,
            JSON_CONTEXT.key_to_cstr(key_with_opt_null),
            JSON_CONTEXT.cstr_len(key_with_opt_null)
        );
        //  d!(> TODO: g_err |= cbor_encode_int(&object##_map, value));
        cbor_encode_int(
            encoder,
            value
        );
    });
    "-------------------------------------------------------------";
}

///////////////////////////////////////////////////////////////////////////////
//  Network Task

///  Compose a CoAP message (CBOR or JSON) with the sensor value in `val` and transmit to the
///  Collector Node (if this is a Sensor Node) or to the CoAP Server (if this is a Collector Node
///  or Standalone Node).  
///  For Sensor Node or Standalone Node: sensor_node is the sensor name (`bme280_0` or `temp_stm32_0`)
///  For Collector Node: sensor_node is the Sensor Node Address of the Sensor Node that transmitted
///  the sensor data (like `b3b4b5b6f1`)
///  The message will be enqueued for transmission by the CoAP / OIC Background Task 
///  so this function will return without waiting for the message to be transmitted.  
///  Return 0 if successful, SYS_EAGAIN if network is not ready yet.
pub fn send_sensor_data(sensor_val: &SensorValue, sensor_node: &CStr) -> MynewtResult<()>  {  //  Returns an error code upon error.
    console_print(b"send_sensor_data\n");
    //  TODO: Remove val
    let mut val = fill_zero!(sensor_value);
    //  For Sensor Node: Transmit the sensor data to the Collector Node as CBOR.
    if unsafe { sensor_network::should_send_to_collector(&mut val, sensor_node.as_ptr()) } { 
        return send_sensor_data_to_collector(sensor_val, sensor_node); 
    }
    //  For Collector Node and Standalone Node: Transmit the sensor data to the CoAP Server as CoAP JSON.
    send_sensor_data_to_server(sensor_val, sensor_node)
}

///////////////////////////////////////////////////////////////////////////////
//  For Collector Node or Standalone Node: Send Sensor Data to CoAP Server (ESP8266)

///  Compose a CoAP JSON message with the Sensor Key (field name) and Value in val 
///  and send to the CoAP server and URI.  The Sensor Value may be integer or float.
///  For temperature, the Sensor Key is either `t` for raw temperature (integer, from 0 to 4095) 
///  or `tmp` for computed temperature (float).
///  The message will be enqueued for transmission by the CoAP / OIC 
///  Background Task so this function will return without waiting for the message 
///  to be transmitted.  Return 0 if successful, `SYS_EAGAIN` if network is not ready yet.
///  For the CoAP server hosted at thethings.io, the CoAP payload should be encoded in JSON like this:
///  ```
///  {"values":[
///    {"key":"device", "value":"0102030405060708090a0b0c0d0e0f10"},
///    {"key":"tmp",    "value":28.7},
///    {"key":"...",    "value":... },
///    ... ]}
///  ```
fn send_sensor_data_to_server(sensor_val: &SensorValue, node_id: &CStr) -> MynewtResult<()>  {  //  Returns an error code upon error.
    if let SensorValueType::None = sensor_val.val { assert!(false); }
    //  assert!(node_id.to_str().unwrap().len() > 0);
    assert_ne!(node_id.to_bytes()[0], 0);
    if unsafe { !NETWORK_IS_READY } { return Err(MynewtError::SYS_EAGAIN); }  //  If network is not ready, tell caller (Sensor Listener) to try later.
    let device_id_ptr = unsafe { sensor_network::get_device_id() };
    let device_id: &CStr = unsafe { CStr::from_ptr(device_id_ptr) };

    //  Start composing the CoAP Server message with the sensor data in the payload.  This will 
    //  block other tasks from composing and posting CoAP messages (through a semaphore).
    //  We only have 1 memory buffer for composing CoAP messages so it needs to be locked.
    let rc = sensor_network::init_server_post(strn!("")) ? ;
    assert!(rc);

    //  Compose the CoAP Payload in JSON using the coap!() macro.
    let _payload = coap!(@json {
        //  Create "values" as an array of items under the root.
        //  Append to the "values" array:
        //    {"key":"device", "value":"0102030405060708090a0b0c0d0e0f10"},
        "device": device_id,
        //    {"key":"node", "value":"b3b4b5b6f1"},
        "node":   node_id,
        //  Append to the "values" array the Sensor Key and Sensor Value, depending on the value type:
        //    {"key":"t",   "value":2870} for raw temperature (integer)
        //    {"key":"tmp", "value":28.7} for computed temperature (float)
        sensor_val,
    });

    //  Post the CoAP Server message to the CoAP Background Task for transmission.  After posting the
    //  message to the background task, we release a semaphore that unblocks other requests
    //  to compose and post CoAP messages.
    let rc =sensor_network::do_server_post() ? ;
    assert!(rc);

    console_print(b"NET view your sensor at \nhttps://blue-pill-geolocate.appspot.com?device=%s\n");  //  , device_id);
    //  console_printf("NET send data: tmp "); console_printfloat(tmp); console_printf("\n");  ////

    //  The CoAP Background Task will call oc_tx_ucast() in the ESP8266 driver to 
    //  transmit the message: libs/esp8266/src/transport.cpp
    Ok(())
}

///////////////////////////////////////////////////////////////////////////////
//  For Sensor Node: Send Sensor Data to Collector Node (nRF24L01)

///  Compose a CoAP CBOR message with the Sensor Key (field name) and Value in val and 
///  transmit to the Collector Node.  The Sensor Value should be integer not float since
///  we transmit integers only to the Collector Node.
///  For temperature, the Sensor Key is `t` for raw temperature (integer, from 0 to 4095).
///  The message will be enqueued for transmission by the CoAP / OIC 
///  Background Task so this function will return without waiting for the message 
///  to be transmitted.  Return 0 if successful, `SYS_EAGAIN` if network is not ready yet.
///  The CoAP payload needs to be very compact (under 32 bytes) so it will be encoded in CBOR like this:
///  `{ t: 2870 }`
fn send_sensor_data_to_collector(sensor_val: &SensorValue, _node_id: &CStr) -> MynewtResult<()>  {  //  Returns an error code upon error.
    ////  TODO: if let SensorValueType::None = sensor_val.val { assert!(false); }
    if unsafe { !NETWORK_IS_READY } { return Err(MynewtError::SYS_EAGAIN); }  //  If network is not ready, tell caller (Sensor Listener) to try later.

    //  Start composing the CoAP Collector message with the sensor data in the payload.  This will 
    //  block other tasks from composing and posting CoAP messages (through a semaphore).
    //  We only have 1 memory buffer for composing CoAP messages so it needs to be locked.
    let rc = unsafe { sensor_network::init_collector_post() };  assert!(rc);

    //  Compose the CoAP Payload in CBOR using the `coap!()` macro.
    let _payload = coap!(@cbor {
        //  Set the Sensor Key and integer Sensor Value, e.g. `{ t: 2870 }`
        sensor_val,
    });

    //  Post the CoAP Collector message to the CoAP Background Task for transmission.  After posting the
    //  message to the background task, we release a semaphore that unblocks other requests
    //  to compose and post CoAP messages.
    let rc = unsafe { sensor_network::do_collector_post() };  assert!(rc);

    console_print(b"NRF send to collector: rawtmp %d\n");  //  , val->int_val);  ////

    //  The CoAP Background Task will call oc_tx_ucast() in the nRF24L01 driver to 
    //  transmit the message: libs/nrf24l01/src/transport.cpp
    Ok(())
}

/*
fn test_json() {
  let device_id = CStr::from_bytes_with_nul(b"0102030405060708090a0b0c0d0e0f10\0").unwrap();
  let node_id   = CStr::from_bytes_with_nul(b"b3b4b5b6f1\0").unwrap();
  //  Sensor `t` has int value 2870.
  let int_sensor_value = SensorValue {
    key: "t",
    val: SensorValueType::Uint(2870)
  };
  //let mut ptr: *mut ::core::ffi::c_void = &mut context as *mut ::core::ffi::c_void;
  //let ptr: *mut c_void = context.to_void_ptr();
  //trace_macros!(true);
  //let a = stringify_null!(device);

  //json_rep_set_text_string!(JSON_CONTEXT, device1, device_id);

  //json_rep_set_text_string!(JSON_CONTEXT, "device2", device_id);

  //coap_item_str! (@json JSON_CONTEXT, "device", device_id);

  //coap_set_int_val! (@json JSON_CONTEXT, int_sensor_value);

  /*
  coap_array! (@json JSON_CONTEXT, values, {  //  Create "values" as an array of items under the root
    coap_item_str! (@json JSON_CONTEXT, "device", device_id);
    coap_item_str! (@json JSON_CONTEXT, "node", node_id);
    coap_set_int_val! (@json JSON_CONTEXT, int_sensor_value);
  });
  */

  trace_macros!(false);
}

fn send_sensor_data_cbor() {
  //  Sensor `t` has int value 2870.
  let int_sensor_value = SensorValue {
    key: "t",
    val: SensorValueType::Uint(2870)
  };
  //  Compose the CoAP Payload in CBOR using the `coap` macro.
  trace_macros!(true);
  let payload = coap!(@cbor {
    int_sensor_value,    //  Send `{t: 2870}`
  });
  trace_macros!(false);
}

///  Null-terminated string "t".
const int_sensor_key: &'static [u8] = b"t\0";
///  Null-terminated string "tmp".
const float_sensor_key: &'static [u8] = b"tmp\0";
const k: &'static [u8] = b"t\0";
const k2: &'static str = "t";
cbor_encode_text_string(&mut root_map,
                        int_sensor_value.key.as_ptr(),
                        int_sensor_value.key.len());
cbor_encode_int(&mut root_map, 1234);
*/

/*
fn send_sensor_data_without_encoding() {
  trace_macros!(true);   //  Start tracing macros
  d!(a b c);             //  Will expand to "a b c" (for debugging)
  trace_macros!(false);  //  Stop tracing macros

  //  Sensor `t` has int value 2870.
  let int_sensor_value = SensorValue {
    key: "t",
    val: SensorValueType::Uint(2870)
  };
  let device_id = b"0102030405060708090a0b0c0d0e0f10";
  let node_id =   b"b3b4b5b6f1";

  //  Compose the CoAP Payload without encoding using the `coap` macro.
  trace_macros!(true);   //  Start tracing macros
  let payload = coap!(@none {
    "device": device_id,
    "node":   node_id,
    int_sensor_value,  //  Send `{t: 2870}`
  });
  trace_macros!(false);  //  Stop tracing macros
}

//  static mut g_encoder: CborEncoder = CborEncoder{};
static mut root_map: CborEncoder = CborEncoder{  //  TODO: Prevent concurrent access.
  writer: 0 as *mut cbor_encoder_writer,
  writer_arg: 0 as *mut ::cty::c_void,
  added: 0,
  flags: 0,
};

fn send_sensor_data_json() {
  let device_id = CStr::from_bytes_with_nul(b"0102030405060708090a0b0c0d0e0f10\0");
  let node_id   = CStr::from_bytes_with_nul(b"b3b4b5b6f1\0");

  //  Sensor `t` has int value 2870.
  let int_sensor_value = SensorValue {
    key: "t",
    val: SensorValueType::Uint(2870)
  };

  //  Compose the CoAP Payload in JSON using the `coap` macro.
  //  trace_macros!(true);
  let payload = coap!(@json {
    "device": device_id,
    //  "node":   node_id,
    //  int_sensor_value,  //  Send `{t: 2870}`
  });
  //  trace_macros!(false);
}

*/