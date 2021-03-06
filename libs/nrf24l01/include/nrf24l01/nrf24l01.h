//  nRF24L01 Driver for Apache Mynewt.  Functions for creating the device instance and performing device functions.
//  More about Mynewt Drivers: https://mynewt.apache.org/latest/os/modules/drivers/driver.html

//  nRF24L01 provides a simple localised network that connects nearby devices and their sensors over 2.4 GHz.
//  In this driver we assume a Star Topology for the network:
//  -- 1 to 5 Sensor Nodes that transmit sensor data
//  -- 1 Collector Node that receives and aggregates sensor data from the Sensor Nodes

//  Each nRF24L01 module can have 1 outgoing pipe for transmitting data and 5 incoming pipes for receiving data
//  -- Collector Node: Will have 5 incoming pipes connected to 5 Sensor Nodes
//  -- Sensor Node: Will have 1 outgoing pipe connected to Collector Node (plus 1 incoming pipe to support acknowledgements in future)

//  Each pipe is identified by a unique address e.g. 0xB3B4B5B6f1
//  All Sensor Nodes must belong to the same 4-byte subnet e.g. 0xB3B4B5B6??
//  Collector Node is allowed to have a different address e.g. 0x7878787878
//  See libs/nrf24l01/src/driver.cpp for actual addresses.

//  We use "Standalone Node" to refer to a node that sends sensor data directly to the CoAP Server
//  via ESP8266, without forwarding to a Collector Node.

#ifndef __NRF24L01_DRIVER_H__
#define __NRF24L01_DRIVER_H__
#include <os/os_dev.h>    //  For os_dev
#include <os/os_mutex.h>  //  For os_mutex
#include <hal/hal_spi.h>  //  For hal_spi_settings

#ifdef __cplusplus
extern "C" {  //  Expose the types and functions below to C functions.
#endif

#define NRF24L01_DEVICE "nrf24l01_0"  //  Name of the device
#define NRL24L01_MAX_RX_PIPES     5   //  Max 5 pipes for receiving data

//  Names (text addresses) of the Sensor Nodes, e.g. "b3b4b5b6f1".  These are also the Remote Sensor names.
#define NRL24L01_MAX_SENSOR_NODE_NAMES NRL24L01_MAX_RX_PIPES  //  Number of Sensor Node names
extern const char *nrf24l01_sensor_node_names[NRL24L01_MAX_SENSOR_NODE_NAMES];
 
//  Device Configuration
struct nrf24l01_cfg {
    struct hal_spi_settings spi_settings;  //  SPI settings
    int spi_num;    //  0 means SPI1, 1 means SPI2
    void *spi_cfg;  //  Low-level MCU SPI config
    int cs_pin;     //  Default is PB2
    int ce_pin;     //  Default is PB0
    int irq_pin;    //  Default is PA15.  Set to MCU_GPIO_PIN_NONE to disable interrupt.
    int freq;       //  Frequency in kHz. Default is 2,476 kHz (channel 76)
    int power;
    int data_rate;
    int crc_width;  //  Default is NRF24L01P_CRC_8_BIT
    //  These settings apply for all pipes.
    int tx_size;
    uint8_t auto_ack;
    uint8_t auto_retransmit;
    //  List of pipes.
    unsigned long long tx_address;     //  Pipe 0
    const unsigned long long *rx_addresses;  //  Pipes 1 to 5
    uint8_t rx_addresses_len;
};

//  Device Instance
struct nrf24l01 {
    struct os_dev dev;
    struct nrf24l01_cfg cfg;
    uint8_t is_configured;  //  0 means not configured
    void *controller;       //  Pointer to controller instance (nRF24L01P *)
};

/////////////////////////////////////////////////////////
//  Device Creation Functions

//  Create the device instance and configure it.  Called by sysinit() during startup, defined in pkg.yml.
//  Implemented in creator.c as function DEVICE_CREATE().
void nrf24l01_create(void);

//  Copy the default device config into cfg.  Returns 0.
int nrf24l01_default_cfg(struct nrf24l01_cfg *cfg);

//  Configure the device.  Called by os_dev_create().  Return 0 if successful.
int nrf24l01_init(struct os_dev *dev0, void *arg);

//  Apply the device configuration.  Return 0 if successful.
int nrf24l01_config(struct nrf24l01 *dev, struct nrf24l01_cfg *cfg);

/////////////////////////////////////////////////////////
//  Transmit / Receive Functions

//  Transmit the data.
int nrf24l01_send(struct nrf24l01 *dev, uint8_t *buf, uint8_t size);

//  Receive data from the pipe.
int nrf24l01_receive(struct nrf24l01 *dev, int pipe, uint8_t *buf, uint8_t size);

//  Return the pipe number that has received data.  -1 if no data received.
int nrf24l01_readable_pipe(struct nrf24l01 *dev);

//  Return the rx address of the pipe (1 to 5).
unsigned long long nrf24l01_get_rx_address(struct nrf24l01 *dev, int pipe);

//  Set the callback function that will be triggered when we receive 
//  an nRF24L01 message. This callback is triggered by the nRF24L01 
//  receive interrupt, which is forwarded to the Default Event Queue.
//  Return 0 if successful.
int nrf24l01_set_rx_callback(struct nrf24l01 *dev, void (*callback)(struct os_event *ev));

/////////////////////////////////////////////////////////
//  Other Functions

//  Flush the transmit buffer.  Return 0 if successful.
int nrf24l01_flush_tx(struct nrf24l01 *dev);

//  Flush the receive buffer.  Return 0 if successful.
int nrf24l01_flush_rx(struct nrf24l01 *dev);

//  Flush the transmit and receive buffers.  Return 0 if successful.
int nrf24l01_flush_txrx(struct nrf24l01 *dev);

#ifdef __cplusplus
}
#endif

#endif /* __NRF24L01_DRIVER_H__ */
