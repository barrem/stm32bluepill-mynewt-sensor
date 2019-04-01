//  Ported from https://github.com/ARMmbed/ATParser//blob/269f14532b98442669c50383782cbce1c67aced5/BufferedSerial/BufferedSerial.cpp
/**
 * @file    BufferedSerial.cpp
 * @brief   Software Buffer - Extends mbed Serial functionallity adding irq driven TX and RX
 * @author  sam grove
 * @version 1.0
 * @see
 *
 * Copyright (c) 2013
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#include <stdarg.h>
#include <assert.h>
#include <os/os.h>
#include <console/console.h>
#include <hal/hal_uart.h>     //  UART functions.
#include "BufferedSerial.h"

extern "C" int BufferedPrintfC(void *stream, int size, const char* format, va_list arg);

#define MY_UART 0  //  Select UART port: 0 means UART2.

static const char *cmds[] = {     //  List of ESP8266 commands to be sent.
    "AT+CWMODE_CUR=3\r\n",  //  Set to WiFi Client mode (not WiFi Access Point mode).
    "AT+CWLAP\r\n",         //  List all WiFi access points.
    NULL                    //  No more commands.
};
static const char **cmd_ptr = NULL;   //  Pointer to ESP8266 command being sent.
static const char *tx_buf = NULL;     //  ESP8266 command buffer being sent.
static const char *tx_ptr = NULL;     //  Pointer to next ESP8266 command buffer byte to be sent.
static char rx_buf[256];        //  ESP8266 receive buffer.
static char *rx_ptr = NULL;     //  Pointer to next ESP8266 receive buffer byte to be received.
static struct os_callout next_cmd_callout;  //  Callout to switch to next ESP8266 command after a delay.

static int uart_tx_char(void *arg) {    
    //  UART driver asks for more data to send. Return -1 if no more data is available for TX.
    if (tx_ptr == NULL || *tx_ptr == 0) { return -1; }
    char byte = *tx_ptr++;  //  Fetch next byte from tx buffer.
    return byte;
}

static int uart_rx_char(void *arg, uint8_t byte) {
    //  UART driver reports incoming byte of data. Return -1 if data was dropped.
    if (rx_ptr - rx_buf < sizeof(rx_buf)) { *rx_ptr++ = byte; }  //  Save to rx buffer.
    return 0;
}

static void uart_tx_done(void *arg) {
    //  UART driver reports that transmission is complete.
    //  We wait 5 seconds for the current command to complete, 
    //  then trigger the next_cmd callout to switch to next ESP8266 command.
    int rc = os_callout_reset(&next_cmd_callout, OS_TICKS_PER_SEC * 5);
    assert(rc == 0);
}

static void next_cmd(struct os_event *ev) {
    //  Switch to next ESP8266 command.
    assert(ev);
    if (rx_buf[0]) {  //  If UART data has been received...
        console_printf("< %s\n", rx_buf);   //  Show the UART data.
        memset(rx_buf, 0, sizeof(rx_buf));  //  Empty the rx buffer.
        rx_ptr = rx_buf;
    }
    if (*cmd_ptr == NULL) {      //  No more commands.
        tx_buf = NULL;
        tx_ptr = NULL;
        return; 
    }
    tx_buf = *cmd_ptr++;         //  Fetch next command.
    tx_ptr = tx_buf;
    hal_uart_start_rx(MY_UART);  //  Start receiving UART data.
    hal_uart_start_tx(MY_UART);  //  Start transmitting UART data.
}

static int setup_uart(void) {
    int rc;
    //  Init tx and rx buffers.
    cmd_ptr = cmds;
    if (*cmd_ptr == NULL) {  //  No more commands.
        tx_buf = NULL;
        tx_ptr = NULL;
        return -1; 
    }
    tx_buf = *cmd_ptr++;  //  Fetch first command.
    tx_ptr = tx_buf;
    memset(rx_buf, 0, sizeof(rx_buf));
    rx_ptr = rx_buf;
    //  Define the next_cmd callout to switch to next ESP8266 command.
    os_callout_init(&next_cmd_callout, os_eventq_dflt_get(), next_cmd, NULL);
    //  Define the UART callbacks.
    rc = hal_uart_init_cbs(MY_UART,
        uart_tx_char, uart_tx_done,
        uart_rx_char, NULL);
    if (rc != 0) { return rc; }
    //  Set UART parameters.
    rc = hal_uart_config(MY_UART,
        115200,
        8,
        1,
        HAL_UART_PARITY_NONE,
        HAL_UART_FLOW_CTL_NONE
    );
    if (rc != 0) { return rc; }
    //  Don't call console_printf() tx/rx or some UART data will be dropped.
    hal_uart_start_rx(MY_UART);  //  Start receiving UART data.
    hal_uart_start_tx(MY_UART);  //  Start transmitting UART data.
    return 0;
}

BufferedSerial::BufferedSerial(uint32_t buf_size, uint32_t tx_multiple, const char* name)
    : _rxbuf(buf_size), _txbuf((uint32_t)(tx_multiple*buf_size))
{
    int rc;
    this->_buf_size = buf_size;
    this->_tx_multiple = tx_multiple;   
    rc = setup_uart();  //  Starting transmitting and receiving to/from the UART port.
    assert(rc == 0);
}

BufferedSerial::~BufferedSerial(void)
{
}

int BufferedSerial::readable(void)
{
    return _rxbuf.available();  // note: look if things are in the buffer
}

int BufferedSerial::writeable(void)
{
    return 1;   // buffer allows overwriting by design, always true
}

int BufferedSerial::getc(void)
{
    return _rxbuf;
}

int BufferedSerial::putc(int c)
{
    _txbuf = (char)c;
    BufferedSerial::prime();

    return c;
}

int BufferedSerial::puts(const char *s)
{
    if (s != NULL) {
        const char* ptr = s;
    
        while(*(ptr) != 0) {
            _txbuf = *(ptr++);
        }
        _txbuf = '\n';  // done per puts definition
        BufferedSerial::prime();
    
        return (ptr - s) + 1;
    }
    return 0;
}

extern "C" size_t BufferedSerialThunk(void *buf_serial, const void *s, size_t length)
{
    BufferedSerial *buffered_serial = (BufferedSerial *)buf_serial;
    return buffered_serial->write(s, length);
}

int BufferedSerial::printf(const char* format, ...)
{
    va_list arg;
    va_start(arg, format);
    int r = BufferedPrintfC((void*)this, this->_buf_size, format, arg);
    va_end(arg);
    return r;
}

size_t BufferedSerial::write(const void *s, size_t length)
{
    if (s != NULL && length > 0) {
        const char* ptr = (const char*)s;
        const char* end = ptr + length;
    
        while (ptr != end) {
            _txbuf = *(ptr++);
        }
        BufferedSerial::prime();
    
        return ptr - (const char*)s;
    }
    return 0;
}


void BufferedSerial::rxIrq(void)
{
    // read from the peripheral and make sure something is available
    if(serial_readable(&_serial)) {
        _rxbuf = serial_getc(&_serial); // if so load them into a buffer
        // trigger callback if necessary
        if (_cbs[RxIrq]) {
            _cbs[RxIrq]();
        }
    }

    return;
}

void BufferedSerial::txIrq(void)
{
    // see if there is room in the hardware fifo and if something is in the software fifo
    while(serial_writable(&_serial)) {
        if(_txbuf.available()) {
            serial_putc(&_serial, (int)_txbuf.get());
        } else {
            // TODO: disable the TX interrupt when there is nothing left to send
            // trigger callback if necessary
            if (_cbs[TxIrq]) {
                _cbs[TxIrq]();
            }
            break;
        }
    }

    return;
}

#ifdef NOTUSED
void BufferedSerial::prime(void)
{
    // if already busy then the irq will pick this up
    if(serial_writable(&_serial)) {
        //  TODO: BufferedSerial::txIrq();                // only write to hardware in one place
    }
    return;
}

void BufferedSerial::attach(Callback<void()> func, IrqType type)
{
    _cbs[type] = func;
}
#endif  //  NOTUSED
